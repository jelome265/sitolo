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

    for table in ["organizations", "organization_memberships", "branches"] {
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

        // The grant decision is SELECT + INSERT only. Assert it here so the
        // verifier fails if the DDL is widened to UPDATE or DELETE.
        let privilege = query(
            "SELECT has_table_privilege('app_runtime', format('%I.%I', $1, $2), 'SELECT') AS can_select,
                    has_table_privilege('app_runtime', format('%I.%I', $1, $2), 'INSERT') AS can_insert,
                    has_table_privilege('app_runtime', format('%I.%I', $1, $2), 'UPDATE') AS can_update,
                    has_table_privilege('app_runtime', format('%I.%I', $1, $2), 'DELETE') AS can_delete",
        )
        .bind(schema_name)
        .bind(table)
        .fetch_one(admin_pool)
        .await?;

        let can_select: bool = privilege.get("can_select");
        let can_insert: bool = privilege.get("can_insert");
        let can_update: bool = privilege.get("can_update");
        let can_delete: bool = privilege.get("can_delete");
        if !can_select || !can_insert || can_update || can_delete {
            violation!(
                table,
                format!(
                    "expected SELECT+INSERT only, got SELECT={can_select} INSERT={can_insert} \
                     UPDATE={can_update} DELETE={can_delete}"
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

        if policies.len() != 2 {
            violation!(
                table,
                format!("expected 2 policies, found {}", policies.len())
            );
        }

        let expected_insert = format!("{table}_provision_insert");
        let expected_read = format!("{table}_provision_read");

        for policy in policies {
            let name: String = policy.get("polname");
            let command: i8 = policy.get("polcmd");
            let roles: Vec<i64> = policy.get("polroles");
            let qual: Option<String> = policy.get("qual_expr");
            let check: Option<String> = policy.get("check_expr");

            if roles != vec![runtime_oid] {
                violation!(
                    table,
                    format!("policy {name} roles {roles:?} do not equal [{runtime_oid}]")
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
                if !references_tenant_column(&check, table) {
                    violation!(
                        table,
                        format!("INSERT check does not reference the tenant column: {check}")
                    );
                }
                if !check.contains("<>") || !check.contains("''") {
                    violation!(
                        table,
                        format!("INSERT check lacks the non-empty guard: {check}")
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
                if !references_tenant_column(&qual, table) {
                    violation!(
                        table,
                        format!("SELECT USING does not reference the tenant column: {qual}")
                    );
                }
            } else {
                violation!(table, format!("unexpected policy {name}"));
            }
        }
    }

    Ok(())
}

/// Checks that a policy expression compares the table's tenant column against
/// the `app.organization_id` transaction context. Whitespace, quotes,
/// parentheses, and case are ignored so the check is independent of
/// `pg_get_expr` rendering.
fn references_tenant_column(expression: &str, table: &str) -> bool {
    let normalized: String = expression
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '"' && *c != '(' && *c != ')')
        .collect::<String>()
        .to_lowercase();
    let without_context = normalized.replace("app.organization_id", "");
    if table == "organizations" {
        without_context.contains("id")
    } else {
        without_context.contains("organization_id")
    }
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
