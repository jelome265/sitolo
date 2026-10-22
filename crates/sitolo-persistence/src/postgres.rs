//! PostgreSQL persistent authority and connection management.
//!
//! Phase 4 Part 7 / PR-008: Manages setup/migration authority separately from
//! the least-privileged runtime authority (`app_runtime`), and enforces trusted
//! transaction-local tenant context derived from `AuthorizedScope`.

use sitolo_domain::tenancy::TenancyError;
use sitolo_tenancy::AuthorizedScope;
use sqlx_core::error::Error;
use sqlx_core::query::query;
use sqlx_core::query_scalar::query_scalar;
use sqlx_core::row::Row;
use sqlx_core::transaction::Transaction;
use sqlx_postgres::{PgConnectOptions, PgPool, PgPoolOptions, Postgres};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PgAuthorityError {
    #[error("database authorization or isolation failure")]
    Sqlx,
    #[error("runtime role security violation")]
    SecurityViolation,
    #[error("tenant domain error: {0}")]
    Domain(#[from] TenancyError),
    #[error("not found or denied by RLS boundary")]
    NotFoundOrDenied,
    #[error(
        "setup failed with {setup_error:?} and secondary cleanup teardown also failed with {teardown_error:?}"
    )]
    ComposedSetupAndTeardownError {
        setup_error: Box<PgAuthorityError>,
        teardown_error: Box<PgAuthorityError>,
    },
}

impl From<Error> for PgAuthorityError {
    fn from(err: Error) -> Self {
        match err {
            Error::RowNotFound => PgAuthorityError::NotFoundOrDenied,
            _ => PgAuthorityError::Sqlx,
        }
    }
}

/// Managed authority pair: admin pool (migration/setup authority) and runtime pool (least privileged).
#[derive(Clone)]
pub struct PgAuthorityPools {
    admin_pool: PgPool,
    runtime_pool: PgPool,
}

impl PgAuthorityPools {
    /// Creates an administrative pool and a runtime pool given database connection options.
    ///
    /// `runtime_opts` must authenticate as the restricted runtime role. The pool does not
    /// assume that role with `SET ROLE`; `verify_runtime_role` fails closed if the login
    /// identity differs.
    pub async fn connect_options(
        admin_opts: PgConnectOptions,
        runtime_opts: PgConnectOptions,
    ) -> Result<Self, PgAuthorityError> {
        let admin_pool = PgPoolOptions::new()
            .max_connections(20)
            .connect_with(admin_opts)
            .await?;

        let runtime_pool = PgPoolOptions::new()
            .max_connections(20)
            .connect_with(runtime_opts)
            .await?;

        Ok(Self {
            admin_pool,
            runtime_pool,
        })
    }

    /// Read-only accessor for runtime pool.
    pub fn runtime_pool(&self) -> &PgPool {
        &self.runtime_pool
    }

    /// Verifies that the runtime pool authenticates as `app_runtime` and that the role has no
    /// administrative privileges, has zero role memberships in `pg_auth_members`,
    /// rolinherit = false, rolreplication = false, and does NOT own protected relations.
    ///
    /// `SET ROLE` is reversible for the remainder of a session, so a pool that logs in as an
    /// administrator and then drops privileges is not a boundary. Both `current_user` and
    /// `session_user` are asserted on the runtime pool itself.
    pub async fn verify_runtime_role(&self, schema_name: &str) -> Result<(), PgAuthorityError> {
        let identity =
            query("SELECT current_user::text AS current_user, session_user::text AS session_user")
                .fetch_one(&self.runtime_pool)
                .await?;

        let current_user: String = identity.get("current_user");
        let session_user: String = identity.get("session_user");

        if current_user != "app_runtime" || session_user != "app_runtime" {
            return Err(PgAuthorityError::SecurityViolation);
        }

        let row = query(
            "SELECT rolsuper, rolbypassrls, rolcreaterole, rolcreatedb, rolinherit, rolreplication
             FROM pg_roles
             WHERE rolname::text = $1",
        )
        .bind(session_user.as_str())
        .fetch_one(&self.admin_pool)
        .await?;

        let superuser: bool = row.get("rolsuper");
        let bypassrls: bool = row.get("rolbypassrls");
        let createrole: bool = row.get("rolcreaterole");
        let createdb: bool = row.get("rolcreatedb");
        let inherit: bool = row.get("rolinherit");
        let replication: bool = row.get("rolreplication");

        if superuser || bypassrls || createrole || createdb || inherit || replication {
            return Err(PgAuthorityError::SecurityViolation);
        }

        // Verify runtime role has NO role memberships in pg_auth_members
        let member_count: i64 = query_scalar(
            "SELECT COUNT(*)
             FROM pg_auth_members m
             JOIN pg_roles r ON r.oid = m.member
             WHERE r.rolname = 'app_runtime'",
        )
        .fetch_one(&self.admin_pool)
        .await?;

        if member_count > 0 {
            return Err(PgAuthorityError::SecurityViolation);
        }

        // Verify runtime role is NOT the owner of protected tables
        let protected_tables = vec!["organizations", "branches", "tenant_resources"];

        for table in protected_tables {
            let owner_role: Option<String> = query_scalar(
                "SELECT pg_get_userbyid(c.relowner)
                 FROM pg_class c
                 JOIN pg_namespace n ON n.oid = c.relnamespace
                 WHERE n.nspname = $1 AND c.relname = $2",
            )
            .bind(schema_name)
            .bind(table)
            .fetch_optional(&self.admin_pool)
            .await?;

            if owner_role.as_deref() == Some("app_runtime") {
                return Err(PgAuthorityError::SecurityViolation);
            }
        }

        Ok(())
    }

    /// Verifies via PostgreSQL catalog functions that `app_runtime` has exact effective privileges
    /// against an explicit allowlist (SELECT, INSERT, UPDATE, DELETE) and no forbidden privileges (CREATE, TRUNCATE, TRIGGER, REFERENCES).
    pub async fn verify_effective_privileges(
        &self,
        schema_name: &str,
    ) -> Result<(), PgAuthorityError> {
        let has_connect: bool = query_scalar(
            "SELECT has_database_privilege('app_runtime', current_database(), 'CONNECT')",
        )
        .fetch_one(&self.admin_pool)
        .await?;

        if !has_connect {
            return Err(PgAuthorityError::SecurityViolation);
        }

        // Assert schema CREATE is FALSE
        let has_schema_create: bool =
            query_scalar("SELECT has_schema_privilege('app_runtime', $1, 'CREATE')")
                .bind(schema_name)
                .fetch_one(&self.admin_pool)
                .await?;

        if has_schema_create {
            return Err(PgAuthorityError::SecurityViolation);
        }

        let has_usage: bool =
            query_scalar("SELECT has_schema_privilege('app_runtime', $1, 'USAGE')")
                .bind(schema_name)
                .fetch_one(&self.admin_pool)
                .await?;

        if !has_usage {
            return Err(PgAuthorityError::SecurityViolation);
        }

        let protected_tables = vec!["organizations", "branches", "tenant_resources"];
        let required_privileges = vec!["SELECT", "INSERT", "UPDATE", "DELETE"];

        for table in protected_tables {
            let rel_name = format!("{schema_name}.{table}");
            for priv_kind in &required_privileges {
                let query = format!(
                    "SELECT has_table_privilege('app_runtime', '{rel_name}', '{priv_kind}')"
                );
                let has_priv: bool = query_scalar(&query).fetch_one(&self.admin_pool).await?;

                if !has_priv {
                    return Err(PgAuthorityError::SecurityViolation);
                }
            }

            let forbidden_privileges = vec!["TRUNCATE", "TRIGGER", "REFERENCES"];
            for priv_kind in &forbidden_privileges {
                let query = format!(
                    "SELECT has_table_privilege('app_runtime', '{rel_name}', '{priv_kind}')"
                );
                let has_priv: bool = query_scalar(&query).fetch_one(&self.admin_pool).await?;

                if has_priv {
                    return Err(PgAuthorityError::SecurityViolation);
                }
            }
        }

        Ok(())
    }

    /// Verifies the **Part 7** RLS fixture (`tests/fixtures/rls_schema.sql`).
    ///
    /// This verifier is bound to that fixture's three tables (`organizations`,
    /// `branches`, `tenant_resources`) and its single `FOR ALL` policy per
    /// table. It must not be used as evidence for the Part 8 provisioning DDL,
    /// which creates a different table set and a different policy contract.
    /// Use [`verify_part8_provisioning_rls_metadata`] for that file.
    pub async fn verify_part7_rls_catalog_metadata(
        &self,
        schema_name: &str,
    ) -> Result<(), PgAuthorityError> {
        let protected_tables = vec!["organizations", "branches", "tenant_resources"];

        for table in protected_tables {
            let row = query(
                "SELECT c.relrowsecurity, c.relforcerowsecurity
                 FROM pg_class c
                 JOIN pg_namespace n ON n.oid = c.relnamespace
                 WHERE n.nspname = $1 AND c.relname = $2",
            )
            .bind(schema_name)
            .bind(table)
            .fetch_one(&self.admin_pool)
            .await?;

            let rowsecurity: bool = row.get("relrowsecurity");
            let forcerowsecurity: bool = row.get("relforcerowsecurity");

            if !rowsecurity || !forcerowsecurity {
                return Err(PgAuthorityError::SecurityViolation);
            }

            // Verify EXACT policy count on relation = 1
            let policy_count: i64 = query_scalar(
                "SELECT COUNT(*)
                 FROM pg_policy p
                 JOIN pg_class c ON c.oid = p.polrelid
                 JOIN pg_namespace n ON n.oid = c.relnamespace
                 WHERE n.nspname = $1 AND c.relname = $2",
            )
            .bind(schema_name)
            .bind(table)
            .fetch_one(&self.admin_pool)
            .await?;

            if policy_count != 1 {
                return Err(PgAuthorityError::SecurityViolation);
            }

            let expected_policy_name = format!("{table}_isolation_policy");
            let policy_row = query(
                "SELECT polname, polcmd, polroles::bigint[] as polroles,
                        pg_get_expr(polqual, polrelid) as qual_expr,
                        pg_get_expr(polwithcheck, polrelid) as check_expr
                 FROM pg_policy p
                 JOIN pg_class c ON c.oid = p.polrelid
                 JOIN pg_namespace n ON n.oid = c.relnamespace
                 WHERE n.nspname = $1 AND c.relname = $2 AND p.polname = $3",
            )
            .bind(schema_name)
            .bind(table)
            .bind(&expected_policy_name)
            .fetch_optional(&self.admin_pool)
            .await?;

            match policy_row {
                Some(p_row) => {
                    let polcmd: i8 = p_row.get::<i8, _>("polcmd");
                    let polroles: Vec<i64> = p_row.get("polroles");
                    let qual_expr: Option<String> = p_row.get("qual_expr");
                    let check_expr: Option<String> = p_row.get("check_expr");

                    // polcmd '*' = ALL in PG catalog pg_policy table
                    if polcmd != b'*' as i8 {
                        return Err(PgAuthorityError::SecurityViolation);
                    }

                    // Verify polroles strictly targets app_runtime role (and NOT 0/PUBLIC, nor empty)
                    let runtime_oid: i64 = query_scalar(
                        "SELECT oid::bigint FROM pg_roles WHERE rolname = 'app_runtime'",
                    )
                    .fetch_one(&self.admin_pool)
                    .await?;

                    if polroles != vec![runtime_oid] {
                        return Err(PgAuthorityError::SecurityViolation);
                    }

                    let qual = qual_expr.ok_or(PgAuthorityError::SecurityViolation)?;
                    let check = check_expr.ok_or(PgAuthorityError::SecurityViolation)?;

                    // Verify exact normalized SQL expressions per table
                    // Normalizes whitespace, quotes, and case without stripping parentheses (boolean grouping structure)
                    fn normalize_expr(s: &str) -> String {
                        s.chars()
                            .filter(|c| !c.is_whitespace() && *c != '"')
                            .collect::<String>()
                            .to_lowercase()
                    }

                    let norm_qual = normalize_expr(&qual);
                    let norm_check = normalize_expr(&check);

                    let expected_canonical_norm = match table {
                        "organizations" => normalize_expr(
                            "((id)::text = NULLIF(current_setting('app.organization_id'::text, true), ''::text))",
                        ),
                        "branches" => normalize_expr(
                            "(((organization_id)::text = NULLIF(current_setting('app.organization_id'::text, true), ''::text)) AND ((NULLIF(current_setting('app.branch_id'::text, true), ''::text) IS NULL) OR ((id)::text = current_setting('app.branch_id'::text, true))))",
                        ),
                        "tenant_resources" => normalize_expr(
                            "(((organization_id)::text = NULLIF(current_setting('app.organization_id'::text, true), ''::text)) AND ((NULLIF(current_setting('app.branch_id'::text, true), ''::text) IS NULL) OR ((branch_id)::text = current_setting('app.branch_id'::text, true))))",
                        ),
                        _ => return Err(PgAuthorityError::SecurityViolation),
                    };

                    if norm_qual != expected_canonical_norm || norm_check != expected_canonical_norm
                    {
                        eprintln!(
                            "RLS policy expression mismatch for table {table}:\nGot qual raw: {qual}\nGot qual norm: {norm_qual}\nExpected norm: {expected_canonical_norm}"
                        );
                        return Err(PgAuthorityError::SecurityViolation);
                    }
                }
                None => {
                    return Err(PgAuthorityError::SecurityViolation);
                }
            }
        }

        Ok(())
    }
}

/// Verifies the **Part 8** provisioning DDL
/// (`schema/phase4_part8_tenancy_provisioning.sql`) against the schema it
/// actually creates.
///
/// The contract differs from the Part 7 fixture by design: the runtime role may
/// read its own organization and branch rows, so each of `organizations`,
/// `organization_memberships`, and `branches` has exactly two policies —
/// `*_provision_insert` (INSERT, `WITH CHECK` only) and `*_provision_read`
/// (SELECT, `USING` only) — both restricted to `app_runtime`. No policy is
/// expected for `tenant_resources`; that table belongs to the Part 7 fixture.
///
/// Each deparsed expression is compared against its operation-specific
/// canonical tenant predicate after renderer-only normalization. This is an
/// exact token comparison, not a substring check: an expression that drops the
/// `current_setting('app.organization_id', true)` comparison cannot satisfy it.
///
/// The DDL names the membership policies `memberships_provision_*` rather than
/// `organization_memberships_provision_*`, so the verifier carries an explicit
/// table-to-prefix map instead of assuming a naming convention.
pub async fn verify_part8_provisioning_rls_metadata(
    admin_pool: &PgPool,
    schema_name: &str,
) -> Result<(), PgAuthorityError> {
    macro_rules! violation {
        ($table:expr, $reason:expr) => {{
            eprintln!("Part 8 RLS contract violation on {}: {}", $table, $reason);
            return Err(PgAuthorityError::SecurityViolation);
        }};
    }

    let runtime_oid: i64 =
        query_scalar("SELECT oid::bigint FROM pg_roles WHERE rolname = 'app_runtime'")
            .fetch_one(admin_pool)
            .await?;
    let outbox_owner_oid: i64 =
        query_scalar("SELECT oid::bigint FROM pg_roles WHERE rolname = 'app_outbox_owner'")
            .fetch_one(admin_pool)
            .await?;

    for (table, policy_prefix, tenant_column, owner_transition_policy) in [
        (
            "organizations",
            "organizations",
            "id",
            Some("organizations_transition_update"),
        ),
        (
            "organization_memberships",
            "memberships",
            "organization_id",
            None,
        ),
        (
            "branches",
            "branches",
            "organization_id",
            Some("branches_transition_update"),
        ),
    ] {
        let row = match query(
            "SELECT c.relrowsecurity, c.relforcerowsecurity
             FROM pg_class c
             JOIN pg_namespace n ON n.oid = c.relnamespace
             WHERE n.nspname = $1 AND c.relname = $2",
        )
        .bind(schema_name)
        .bind(table)
        .fetch_optional(admin_pool)
        .await?
        {
            Some(row) => row,
            None => violation!(table, "table not found"),
        };

        let rowsecurity: bool = row.get("relrowsecurity");
        let forcerowsecurity: bool = row.get("relforcerowsecurity");
        if !rowsecurity || !forcerowsecurity {
            violation!(
                table,
                format!("RLS enabled={rowsecurity} forced={forcerowsecurity}")
            );
        }

        // Runtime receives only the provisioning grants and the narrowly
        // scoped membership state columns needed for approved transitions.
        let privilege = query(
            "SELECT has_table_privilege('app_runtime', format('%I.%I', $1, $2), 'SELECT') AS can_select,
                    has_table_privilege('app_runtime', format('%I.%I', $1, $2), 'INSERT') AS can_insert,
                    has_table_privilege('app_runtime', format('%I.%I', $1, $2), 'UPDATE') AS can_update,
                    has_table_privilege('app_runtime', format('%I.%I', $1, $2), 'DELETE') AS can_delete,
                    has_column_privilege('app_runtime', format('%I.%I', $1, $2), 'state', 'UPDATE') AS can_update_state,
                    has_column_privilege('app_runtime', format('%I.%I', $1, $2), 'state_version', 'UPDATE') AS can_update_version,
                    has_column_privilege('app_runtime', format('%I.%I', $1, $2), 'id', 'UPDATE') AS can_update_id,
                    CASE WHEN $2 = 'organization_memberships'
                         THEN has_column_privilege('app_runtime', format('%I.%I', $1, $2), 'organization_id', 'UPDATE')
                         ELSE false END AS can_update_organization,
                    CASE WHEN $2 = 'organization_memberships'
                         THEN has_column_privilege('app_runtime', format('%I.%I', $1, $2), 'user_id', 'UPDATE')
                         ELSE false END AS can_update_user",
        )
        .bind(schema_name)
        .bind(table)
        .fetch_one(admin_pool)
        .await?;

        let can_select: bool = privilege.get("can_select");
        let can_insert: bool = privilege.get("can_insert");
        let can_update: bool = privilege.get("can_update");
        let can_delete: bool = privilege.get("can_delete");
        let can_update_state: bool = privilege.get("can_update_state");
        let can_update_version: bool = privilege.get("can_update_version");
        let can_update_id: bool = privilege.get("can_update_id");
        let can_update_organization: bool = privilege.get("can_update_organization");
        let can_update_user: bool = privilege.get("can_update_user");
        let membership_table = table == "organization_memberships";
        if !can_select
            || !can_insert
            || can_update
            || can_delete
            || can_update_state != membership_table
            || can_update_version != membership_table
            || can_update_id
            || can_update_organization
            || can_update_user
        {
            violation!(
                table,
                format!(
                    "unexpected runtime grants: SELECT={can_select} INSERT={can_insert} \
                     table UPDATE={can_update} DELETE={can_delete} \
                     state UPDATE={can_update_state} version UPDATE={can_update_version} \
                     id UPDATE={can_update_id} organization UPDATE={can_update_organization} \
                     user UPDATE={can_update_user}"
                )
            );
        }

        let policies = query(
            "SELECT p.polname, p.polcmd, p.polroles::bigint[] AS polroles,
                    pg_get_expr(p.polqual, p.polrelid) AS qual_expr,
                    pg_get_expr(p.polwithcheck, p.polrelid) AS check_expr
             FROM pg_policy p
             JOIN pg_class c ON c.oid = p.polrelid
             JOIN pg_namespace n ON n.oid = c.relnamespace
             WHERE n.nspname = $1 AND c.relname = $2
             ORDER BY p.polname",
        )
        .bind(schema_name)
        .bind(table)
        .fetch_all(admin_pool)
        .await?;

        let expected_policy_count = if owner_transition_policy.is_some() {
            4
        } else {
            3
        };
        if policies.len() != expected_policy_count {
            violation!(
                table,
                format!(
                    "expected {expected_policy_count} policies, found {}",
                    policies.len()
                )
            );
        }

        let expected_insert = format!("{policy_prefix}_provision_insert");
        let expected_read = format!("{policy_prefix}_provision_read");
        let expected_owner_read = format!("{policy_prefix}_transition_read");
        let expected_update = "memberships_transition_update";
        let expected_owner_update = normalize_policy_expression(&format!(
            "{tenant_column} = NULLIF(current_setting('app.organization_id', true), '') \
             AND current_setting('app.organization_id', true) <> ''"
        ));
        let expected_insert_predicate = normalize_policy_expression(&format!(
            "{tenant_column} = NULLIF(current_setting('app.organization_id', true), '') \
             AND current_setting('app.organization_id', true) <> ''"
        ));
        let expected_select_predicate = normalize_policy_expression(&format!(
            "{tenant_column} = NULLIF(current_setting('app.organization_id', true), '')"
        ));

        for policy in policies {
            let name: String = policy.get("polname");
            let command: i8 = policy.get("polcmd");
            let roles: Vec<i64> = policy.get("polroles");
            let qual: Option<String> = policy.get("qual_expr");
            let check: Option<String> = policy.get("check_expr");

            let is_owner_transition =
                owner_transition_policy.is_some_and(|expected| name == expected);
            let is_owner_read = owner_transition_policy.is_some() && name == expected_owner_read;
            let expected_role = if is_owner_transition || is_owner_read {
                outbox_owner_oid
            } else {
                runtime_oid
            };
            if roles != vec![expected_role] {
                violation!(
                    table,
                    format!("policy {name} roles {roles:?} do not equal [{expected_role}]")
                );
            }

            if name == expected_insert {
                // INSERT has no USING clause; the row must pass WITH CHECK.
                if command != b'a' as i8 || qual.is_some() {
                    violation!(
                        table,
                        format!(
                            "INSERT policy {name} command={command} using={:?}",
                            qual.as_deref()
                        )
                    );
                }
                let check = match check {
                    Some(value) => value,
                    None => violation!(table, format!("INSERT policy {name} has no WITH CHECK")),
                };
                let actual = normalize_policy_expression(&check);
                if actual != expected_insert_predicate {
                    violation!(
                        table,
                        format!(
                            "INSERT WITH CHECK differs from the canonical tenant predicate; \
                             expected normalized `{expected_insert_predicate}`, got `{actual}` \
                             (raw: {check})"
                        )
                    );
                }
            } else if name == expected_read {
                // SELECT has no WITH CHECK clause; USING limits visible rows.
                if command != b'r' as i8 || check.is_some() {
                    violation!(
                        table,
                        format!(
                            "SELECT policy {name} command={command} with_check={:?}",
                            check.as_deref()
                        )
                    );
                }
                let qual = match qual {
                    Some(value) => value,
                    None => violation!(table, format!("SELECT policy {name} has no USING")),
                };
                let actual = normalize_policy_expression(&qual);
                if actual != expected_select_predicate {
                    violation!(
                        table,
                        format!(
                            "SELECT USING differs from the canonical tenant predicate; \
                             expected normalized `{expected_select_predicate}`, got `{actual}` \
                             (raw: {qual})"
                        )
                    );
                }
            } else if is_owner_read {
                if command != b'r' as i8 || check.is_some() {
                    violation!(
                        table,
                        format!(
                            "owner SELECT policy {name} command={command} with_check={:?}",
                            check.as_deref()
                        )
                    );
                }
                let qual = match qual {
                    Some(value) => value,
                    None => violation!(table, format!("owner SELECT policy {name} has no USING")),
                };
                if normalize_policy_expression(&qual) != expected_owner_update {
                    violation!(
                        table,
                        format!("owner SELECT policy {name} does not enforce the tenant context")
                    );
                }
            } else if membership_table && name == expected_update {
                let expected_update_predicate = normalize_policy_expression(&format!(
                    "{tenant_column} = NULLIF(current_setting('app.organization_id', true), '') \
                     AND current_setting('app.organization_id', true) <> ''"
                ));
                if command != b'w' as i8 {
                    violation!(table, format!("UPDATE policy {name} command={command}"));
                }
                let qual = match qual {
                    Some(value) => value,
                    None => violation!(table, format!("UPDATE policy {name} has no USING")),
                };
                let check = match check {
                    Some(value) => value,
                    None => violation!(table, format!("UPDATE policy {name} has no WITH CHECK")),
                };
                if normalize_policy_expression(&qual) != expected_select_predicate
                    || normalize_policy_expression(&check) != expected_update_predicate
                {
                    violation!(
                        table,
                        format!("UPDATE policy {name} does not enforce the tenant context")
                    );
                }
            } else if is_owner_transition {
                if command != b'w' as i8 {
                    violation!(table, format!("UPDATE policy {name} command={command}"));
                }
                let qual = match qual {
                    Some(value) => value,
                    None => violation!(table, format!("UPDATE policy {name} has no USING")),
                };
                let check = match check {
                    Some(value) => value,
                    None => violation!(table, format!("UPDATE policy {name} has no WITH CHECK")),
                };
                if normalize_policy_expression(&qual) != expected_select_predicate
                    || normalize_policy_expression(&check) != expected_owner_update
                {
                    violation!(
                        table,
                        format!("UPDATE policy {name} does not enforce the owner tenant context")
                    );
                }
            } else {
                violation!(table, format!("unexpected policy {name}"));
            }
        }
    }

    let role_table = match query(
        "SELECT c.relrowsecurity, c.relforcerowsecurity
         FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
         WHERE n.nspname = $1 AND c.relname = 'role_assignments'",
    )
    .bind(schema_name)
    .fetch_optional(admin_pool)
    .await?
    {
        Some(row) => row,
        None => violation!("role_assignments", "table not found"),
    };
    let role_rls: bool = role_table.get("relrowsecurity");
    let role_force_rls: bool = role_table.get("relforcerowsecurity");
    if !role_rls || !role_force_rls {
        violation!(
            "role_assignments",
            format!("RLS enabled={role_rls} forced={role_force_rls}")
        );
    }
    let role_privileges = query(
        "SELECT has_table_privilege('app_runtime', format('%I.%I', $1, 'role_assignments'), 'SELECT') AS can_select,
                has_table_privilege('app_runtime', format('%I.%I', $1, 'role_assignments'), 'INSERT') AS can_insert,
                has_table_privilege('app_runtime', format('%I.%I', $1, 'role_assignments'), 'UPDATE') AS can_update,
                has_table_privilege('app_runtime', format('%I.%I', $1, 'role_assignments'), 'DELETE') AS can_delete,
                has_column_privilege('app_runtime', format('%I.%I', $1, 'role_assignments'), 'state', 'UPDATE') AS can_update_state,
                has_column_privilege('app_runtime', format('%I.%I', $1, 'role_assignments'), 'state_version', 'UPDATE') AS can_update_version,
                has_column_privilege('app_runtime', format('%I.%I', $1, 'role_assignments'), 'id', 'UPDATE') AS can_update_id",
    )
    .bind(schema_name)
    .fetch_one(admin_pool)
    .await?;
    let can_select: bool = role_privileges.get("can_select");
    let can_insert: bool = role_privileges.get("can_insert");
    let can_update: bool = role_privileges.get("can_update");
    let can_delete: bool = role_privileges.get("can_delete");
    let can_update_state: bool = role_privileges.get("can_update_state");
    let can_update_version: bool = role_privileges.get("can_update_version");
    let can_update_id: bool = role_privileges.get("can_update_id");
    if !can_select
        || !can_insert
        || can_update
        || can_delete
        || !can_update_state
        || !can_update_version
        || can_update_id
    {
        violation!("role_assignments", "unexpected app_runtime privileges");
    }
    let role_policy = query(
        "SELECT p.polname, p.polcmd, p.polroles::bigint[] AS polroles,
                pg_get_expr(p.polqual, p.polrelid) AS qual_expr,
                pg_get_expr(p.polwithcheck, p.polrelid) AS check_expr
         FROM pg_policy p JOIN pg_class c ON c.oid = p.polrelid
         JOIN pg_namespace n ON n.oid = c.relnamespace
         WHERE n.nspname = $1 AND c.relname = 'role_assignments'",
    )
    .bind(schema_name)
    .fetch_all(admin_pool)
    .await?;
    if role_policy.len() != 1 {
        violation!(
            "role_assignments",
            format!("expected one tenant policy, found {}", role_policy.len())
        );
    }
    let policy = &role_policy[0];
    let policy_name: String = policy.get("polname");
    let policy_command: i8 = policy.get("polcmd");
    let policy_roles: Vec<i64> = policy.get("polroles");
    let policy_qual: Option<String> = policy.get("qual_expr");
    let policy_check: Option<String> = policy.get("check_expr");
    let tenant_predicate = normalize_policy_expression(
        "organization_id = NULLIF(current_setting('app.organization_id', true), '')",
    );
    let tenant_check = normalize_policy_expression(
        "organization_id = NULLIF(current_setting('app.organization_id', true), '') AND current_setting('app.organization_id', true) <> ''",
    );
    if policy_name != "role_assignments_tenant_all"
        || policy_command != b'*' as i8
        || policy_roles != vec![runtime_oid]
        || policy_qual
            .as_deref()
            .is_none_or(|expr| normalize_policy_expression(expr) != tenant_predicate)
        || policy_check
            .as_deref()
            .is_none_or(|expr| normalize_policy_expression(expr) != tenant_check)
    {
        violation!("role_assignments", "tenant policy differs from contract");
    }

    let scope_metadata = match query(
        "SELECT c.relrowsecurity, c.relforcerowsecurity,
                has_table_privilege('app_runtime', format('%I.%I', $1, 'scope_grants'), 'SELECT') AS can_select,
                has_table_privilege('app_runtime', format('%I.%I', $1, 'scope_grants'), 'INSERT') AS can_insert,
                has_table_privilege('app_runtime', format('%I.%I', $1, 'scope_grants'), 'UPDATE') AS can_update,
                has_table_privilege('app_runtime', format('%I.%I', $1, 'scope_grants'), 'DELETE') AS can_delete,
                has_column_privilege('app_runtime', format('%I.%I', $1, 'scope_grants'), 'state', 'UPDATE') AS can_update_state,
                has_column_privilege('app_runtime', format('%I.%I', $1, 'scope_grants'), 'state_version', 'UPDATE') AS can_update_version,
                has_column_privilege('app_runtime', format('%I.%I', $1, 'scope_grants'), 'id', 'UPDATE') AS can_update_id
         FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
         WHERE n.nspname = $1 AND c.relname = 'scope_grants'",
    )
    .bind(schema_name)
    .fetch_optional(admin_pool)
    .await?
    {
        Some(row) => row,
        None => violation!("scope_grants", "table not found"),
    };
    let scope_rls: bool = scope_metadata.get("relrowsecurity");
    let scope_force_rls: bool = scope_metadata.get("relforcerowsecurity");
    let scope_select: bool = scope_metadata.get("can_select");
    let scope_insert: bool = scope_metadata.get("can_insert");
    let scope_update: bool = scope_metadata.get("can_update");
    let scope_delete: bool = scope_metadata.get("can_delete");
    let scope_update_state: bool = scope_metadata.get("can_update_state");
    let scope_update_version: bool = scope_metadata.get("can_update_version");
    let scope_update_id: bool = scope_metadata.get("can_update_id");
    if !scope_rls
        || !scope_force_rls
        || !scope_select
        || !scope_insert
        || scope_update
        || scope_delete
        || !scope_update_state
        || !scope_update_version
        || scope_update_id
    {
        violation!(
            "scope_grants",
            "RLS or app_runtime privileges differ from contract"
        );
    }
    let scope_policy = query(
        "SELECT p.polname, p.polcmd, p.polroles::bigint[] AS polroles,
                pg_get_expr(p.polqual, p.polrelid) AS qual_expr,
                pg_get_expr(p.polwithcheck, p.polrelid) AS check_expr
         FROM pg_policy p JOIN pg_class c ON c.oid = p.polrelid
         JOIN pg_namespace n ON n.oid = c.relnamespace
         WHERE n.nspname = $1 AND c.relname = 'scope_grants'",
    )
    .bind(schema_name)
    .fetch_all(admin_pool)
    .await?;
    if scope_policy.len() != 1 {
        violation!("scope_grants", "expected one tenant policy");
    }
    let policy = &scope_policy[0];
    let policy_name: String = policy.get("polname");
    let policy_command: i8 = policy.get("polcmd");
    let policy_roles: Vec<i64> = policy.get("polroles");
    let policy_qual: Option<String> = policy.get("qual_expr");
    let policy_check: Option<String> = policy.get("check_expr");
    if policy_name != "scope_grants_tenant_all"
        || policy_command != b'*' as i8
        || policy_roles != vec![runtime_oid]
        || policy_qual
            .as_deref()
            .is_none_or(|expr| normalize_policy_expression(expr) != tenant_predicate)
        || policy_check
            .as_deref()
            .is_none_or(|expr| normalize_policy_expression(expr) != tenant_check)
    {
        violation!("scope_grants", "tenant policy differs from contract");
    }

    let invitation_metadata = match query(
        "SELECT c.relrowsecurity, c.relforcerowsecurity,
                has_table_privilege('app_runtime', format('%I.%I', $1, 'invitations'), 'SELECT') AS can_select,
                has_table_privilege('app_runtime', format('%I.%I', $1, 'invitations'), 'INSERT') AS can_insert,
                has_table_privilege('app_runtime', format('%I.%I', $1, 'invitations'), 'UPDATE') AS can_update,
                has_table_privilege('app_runtime', format('%I.%I', $1, 'invitations'), 'DELETE') AS can_delete,
                has_column_privilege('app_runtime', format('%I.%I', $1, 'invitations'), 'state', 'UPDATE') AS can_update_state,
                has_column_privilege('app_runtime', format('%I.%I', $1, 'invitations'), 'state_version', 'UPDATE') AS can_update_version,
                has_column_privilege('app_runtime', format('%I.%I', $1, 'invitations'), 'contact_kind', 'UPDATE') AS can_update_contact_kind,
                has_column_privilege('app_runtime', format('%I.%I', $1, 'invitations'), 'contact_value', 'UPDATE') AS can_update_contact_value,
                has_column_privilege('app_runtime', format('%I.%I', $1, 'invitations'), 'token_hash', 'UPDATE') AS can_update_token_hash,
                has_column_privilege('app_runtime', format('%I.%I', $1, 'invitations'), 'id', 'UPDATE') AS can_update_id
         FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
         WHERE n.nspname = $1 AND c.relname = 'invitations'",
    )
    .bind(schema_name)
    .fetch_optional(admin_pool)
    .await?
    {
        Some(row) => row,
        None => violation!("invitations", "table not found"),
    };
    let invitation_rls: bool = invitation_metadata.get("relrowsecurity");
    let invitation_force_rls: bool = invitation_metadata.get("relforcerowsecurity");
    let invitation_select: bool = invitation_metadata.get("can_select");
    let invitation_insert: bool = invitation_metadata.get("can_insert");
    let invitation_update: bool = invitation_metadata.get("can_update");
    let invitation_delete: bool = invitation_metadata.get("can_delete");
    let invitation_state: bool = invitation_metadata.get("can_update_state");
    let invitation_version: bool = invitation_metadata.get("can_update_version");
    let invitation_contact_kind: bool = invitation_metadata.get("can_update_contact_kind");
    let invitation_contact_value: bool = invitation_metadata.get("can_update_contact_value");
    let invitation_token_hash: bool = invitation_metadata.get("can_update_token_hash");
    let invitation_id: bool = invitation_metadata.get("can_update_id");
    if !invitation_rls
        || !invitation_force_rls
        || !invitation_select
        || !invitation_insert
        || invitation_update
        || invitation_delete
        || !invitation_state
        || !invitation_version
        || !invitation_contact_kind
        || !invitation_contact_value
        || !invitation_token_hash
        || invitation_id
    {
        violation!(
            "invitations",
            "RLS or app_runtime privileges differ from contract"
        );
    }
    let invitation_policies = query(
        "SELECT p.polname, p.polcmd, p.polroles::bigint[] AS polroles,
                pg_get_expr(p.polqual, p.polrelid) AS qual_expr,
                pg_get_expr(p.polwithcheck, p.polrelid) AS check_expr
         FROM pg_policy p JOIN pg_class c ON c.oid = p.polrelid
         JOIN pg_namespace n ON n.oid = c.relnamespace
         WHERE n.nspname = $1 AND c.relname = 'invitations'",
    )
    .bind(schema_name)
    .fetch_all(admin_pool)
    .await?;
    if invitation_policies.len() != 4 {
        violation!("invitations", "expected four scoped policies");
    }
    let token_resolution_predicate = normalize_policy_expression(
        "state = 'ISSUED' AND token_hash = decode(NULLIF(current_setting('app.invitation_token_hash', true), ''), 'hex')",
    );
    for policy in invitation_policies {
        let name: String = policy.get("polname");
        let command: i8 = policy.get("polcmd");
        let roles: Vec<i64> = policy.get("polroles");
        let qual: Option<String> = policy.get("qual_expr");
        let check: Option<String> = policy.get("check_expr");
        let (expected_name, expected_command, expected_role, expected_qual, expected_check) =
            match name.as_str() {
                "invitations_tenant_insert" => (
                    "invitations_tenant_insert",
                    b'a' as i8,
                    runtime_oid,
                    None,
                    Some(tenant_check.as_str()),
                ),
                "invitations_tenant_read" => (
                    "invitations_tenant_read",
                    b'r' as i8,
                    runtime_oid,
                    Some(tenant_predicate.as_str()),
                    None,
                ),
                "invitations_terminal_update" => (
                    "invitations_terminal_update",
                    b'w' as i8,
                    runtime_oid,
                    Some(tenant_predicate.as_str()),
                    Some(tenant_check.as_str()),
                ),
                "invitations_token_resolution" => (
                    "invitations_token_resolution",
                    b'r' as i8,
                    outbox_owner_oid,
                    Some(token_resolution_predicate.as_str()),
                    None,
                ),
                _ => violation!("invitations", format!("unexpected policy {name}")),
            };
        if name != expected_name
            || command != expected_command
            || roles != vec![expected_role]
            || qual.as_deref().is_none_or(|expr| {
                expected_qual.is_none_or(|expected| normalize_policy_expression(expr) != expected)
            }) != expected_qual.is_none()
            || check.as_deref().is_none_or(|expr| {
                expected_check.is_none_or(|expected| normalize_policy_expression(expr) != expected)
            }) != expected_check.is_none()
        {
            violation!(
                "invitations",
                format!("policy {name} differs from contract")
            );
        }
    }

    Ok(())
}

/// Normalizes a deparsed policy expression for exact comparison.
///
/// The normalization accepts only the renderer-level differences that do not
/// change the fixed predicate shapes verified here: whitespace, double quotes,
/// parentheses, `::text` casts, and the `!=`/`<>` spelling. Case is normalized
/// outside single-quoted literals only, so a changed setting name such as
/// `'APP.ORGANIZATION_ID'` cannot compare equal to the canonical key. It never
/// removes a column, operator, literal, or `current_setting` reference, so a
/// tenant-blind expression cannot compare equal to the canonical form.
fn normalize_policy_expression(expression: &str) -> String {
    let mut normalized = String::with_capacity(expression.len());
    let mut in_literal = false;
    for c in expression.chars() {
        match c {
            '\'' => {
                in_literal = !in_literal;
                normalized.push(c);
            }
            _ if in_literal => normalized.push(c),
            _ => normalized.extend(c.to_lowercase()),
        }
    }
    normalized
        .replace("::text", "")
        .replace("!=", "<>")
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '"' && *c != '(' && *c != ')')
        .collect()
}

/// Sets transaction-local tenant context (`app.organization_id` and `app.branch_id`)
/// derived strictly from `AuthorizedScope`.
pub async fn set_transaction_tenant_context(
    tx: &mut Transaction<'_, Postgres>,
    scope: &AuthorizedScope,
) -> Result<(), PgAuthorityError> {
    query("SELECT set_config('app.organization_id', $1, true)")
        .bind(scope.organization_id().as_str())
        .execute(&mut **tx)
        .await?;

    if let Some(branch_id) = scope.branch_id() {
        query("SELECT set_config('app.branch_id', $1, true)")
            .bind(branch_id.as_str())
            .execute(&mut **tx)
            .await?;
    } else {
        query("SELECT set_config('app.branch_id', '', true)")
            .execute(&mut **tx)
            .await?;
    }

    Ok(())
}

#[cfg(test)]
mod policy_expression_tests {
    use super::normalize_policy_expression;

    #[test]
    fn renderer_equivalent_predicates_normalize_equal() {
        let canonical = normalize_policy_expression(
            "id = NULLIF(current_setting('app.organization_id', true), '')",
        );
        let rendered = normalize_policy_expression(
            "((id)::text = NULLIF(current_setting('app.organization_id'::text, true), ''::text))",
        );
        assert_eq!(canonical, rendered);
    }

    #[test]
    fn tenant_blind_predicates_do_not_normalize_to_the_canonical_form() {
        let canonical = normalize_policy_expression(
            "id = NULLIF(current_setting('app.organization_id', true), '')",
        );
        assert_ne!(normalize_policy_expression("id IS NOT NULL"), canonical);
        assert_ne!(normalize_policy_expression("id <> ''"), canonical);
        assert_ne!(
            normalize_policy_expression(
                "id = NULLIF(current_setting('APP.ORGANIZATION_ID', true), '')"
            ),
            canonical
        );
    }

    #[test]
    fn alternate_inequality_spelling_normalizes_equal() {
        assert_eq!(
            normalize_policy_expression("id != ''"),
            normalize_policy_expression("id <> ''")
        );
    }
}
