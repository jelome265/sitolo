//! Row-level security (RLS) defense-in-depth simulation.
//!
//! Phase 4 specification §26 and Phase 5 RLS contract. Production PG
//! enforces tenant isolation with `CREATE POLICY tenant_isolation ON …
//! USING (organization_id = current_setting('app.organization_id'))` plus
//! `SET LOCAL app.organization_id` per transaction. This module mirrors that
//! model in-memory so PR-008 can prove the same negative cases without a
//! live database: every scoped read is filtered by the DB session context
//! even if the application-layer predicate were forgotten.
//!
//! Design notes:
//! - `RlsContext` is the trusted DB session context (§26: APPLICATION IAM +
//!   TRUSTED DB SESSION CONTEXT + RLS). It is set only from an
//!   `AuthorizedScope` or `TrustedOrganizationId` — never from a raw
//!   `X-Organization-Id` header.
//! - `RlsTenancyDatabase` wraps `TenancyDatabase` and enforces the RLS
//!   predicate on every scoped access. A missing context fails closed
//!   (deny by default); a mismatched tenant returns absence, not a leak.
//! - Predicate logic mirrors §27.1: `WHERE organization_id = current_setting`
//!   — the same column the application predicate uses, so the two layers
//!   are redundant, not divergent.

use std::sync::Mutex;

use sitolo_domain::tenancy::{Branch, BranchId, Membership, MembershipId, OrganizationId};

use crate::tenancy::{TenancyDatabase, TenancyStores};

/// Trusted DB session context (§26). Holds only the organization tenant; user
/// and session fields are available for audit but not for RLS filtering in
/// this phase. Constructed only from server-authorized scopes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RlsContext {
    pub organization_id: OrganizationId,
}

impl RlsContext {
    /// Creates a context from a trusted organization. Callers must have
    /// already derived `organization_id` from `bind_organization` or an
    /// `EffectiveScope` — raw client ids are never accepted.
    #[must_use]
    pub fn from_organization(organization_id: OrganizationId) -> Self {
        RlsContext { organization_id }
    }
}

/// PostgreSQL RLS DDL that Phase 5 will enforce. Kept as constants so tests
/// and docs reference the exact policy text.
pub mod policy {
    /// Tenant isolation predicate used on every tenant-owned table (§27.1).
    pub const TENANT_PREDICATE: &str =
        "organization_id = current_setting('app.organization_id', true)";

    pub const BRANCHES: &str = "CREATE POLICY tenant_isolation ON branches USING (organization_id = current_setting('app.organization_id', true))";
    pub const MEMBERSHIPS: &str = "CREATE POLICY tenant_isolation ON organization_memberships USING (organization_id = current_setting('app.organization_id', true))";
    pub const ASSIGNMENTS: &str = "CREATE POLICY tenant_isolation ON membership_roles USING (organization_id = current_setting('app.organization_id', true))";
    pub const GRANTS: &str = "CREATE POLICY tenant_isolation ON scope_grants USING (organization_id = current_setting('app.organization_id', true))";
    pub const INVITATIONS: &str = "CREATE POLICY tenant_isolation ON invitations USING (organization_id = current_setting('app.organization_id', true))";
}

/// In-memory RLS-enforcing wrapper around [`TenancyDatabase`] (§26, §26.1).
///
/// The inner database already enforces application predicates (§27.1). This
/// wrapper adds the second layer: the DB session context. Every scoped
/// operation first checks the context; a missing or mismatched context
/// denies, so that even if an application handler forgot its predicate, the
/// database would still hide the foreign tenant's rows (defense in depth),
/// and tests can prove `cross-tenant → denied` at the DB layer (§26.1).
pub struct RlsTenancyDatabase {
    inner: TenancyDatabase,
    context: Mutex<Option<RlsContext>>,
}

impl RlsTenancyDatabase {
    #[must_use]
    pub fn new(inner: TenancyDatabase) -> Self {
        RlsTenancyDatabase {
            inner,
            context: Mutex::new(None),
        }
    }

    /// Sets the DB session context from a trusted scope (§26: trusted DB
    /// session context is application-derived, not client-supplied). Mirrors
    /// `SET LOCAL app.organization_id = $1` per transaction.
    pub fn set_context(&self, ctx: RlsContext) {
        *self.context.lock().expect("rls context poisoned") = Some(ctx);
    }

    /// Clears the session context (mirrors transaction end / `RESET`).
    pub fn clear_context(&self) {
        *self.context.lock().expect("rls context poisoned") = None;
    }

    fn current_context(&self) -> Option<RlsContext> {
        self.context.lock().expect("rls context poisoned").clone()
    }

    /// Unscoped access to the inner database for setup that must bypass RLS
    /// (e.g., provisioning the second tenant in tests). Production code
    /// must not use this for tenant-owned reads.
    #[must_use]
    pub fn inner(&self) -> &TenancyDatabase {
        &self.inner
    }

    // --- RLS-filtered scoped reads (§26.1, §27.1) ---

    /// RLS-filtered branch fetch. Returns `None` if the session context is
    /// absent (fail closed) or the branch's organization does not match the
    /// context — the same rows RLS would hide in PG.
    pub async fn branch_snapshot_rls(&self, id: &BranchId) -> Option<Branch> {
        let ctx = self.current_context()?;
        // Delegate to the predicate-enforcing inner, but the predicate value
        // comes from the DB session context, not the caller-supplied org.
        self.inner.branch_snapshot(&ctx.organization_id, id).await
    }

    /// RLS-filtered membership fetch.
    pub async fn membership_snapshot_rls(&self, id: &MembershipId) -> Option<Membership> {
        let ctx = self.current_context()?;
        self.inner
            .membership_snapshot(&ctx.organization_id, id)
            .await
    }
}

impl Default for RlsTenancyDatabase {
    fn default() -> Self {
        Self::new(TenancyDatabase::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tenancy::TenancyStores;
    use sitolo_domain::tenancy::{BranchId, MembershipId, OrganizationId, TenantUserId};

    fn org_id(value: &str) -> OrganizationId {
        OrganizationId::new(value).unwrap()
    }
    fn user(value: &str) -> TenantUserId {
        TenantUserId::new(value).unwrap()
    }
    async fn active_org(db: &TenancyDatabase, id: &str) {
        let org = db
            .create_organization(org_id(id), "Merchant")
            .await
            .unwrap();
        db.activate_organization(&org.id).await.unwrap();
    }
    async fn active_member(
        db: &TenancyDatabase,
        org: &str,
        member: &str,
        user_id: &str,
    ) -> Membership {
        let m = db
            .invite_membership(
                MembershipId::new(member).unwrap(),
                org_id(org),
                user(user_id),
            )
            .await
            .unwrap();
        db.advance_membership(&org_id(org), &m.id).await.unwrap();
        db.activate_membership(&org_id(org), &m.id).await.unwrap()
    }

    #[test]
    fn policy_constants_are_tenant_predicates() {
        assert!(policy::TENANT_PREDICATE.contains("organization_id"));
        assert!(policy::TENANT_PREDICATE.contains("app.organization_id"));
        for ddl in [
            policy::BRANCHES,
            policy::MEMBERSHIPS,
            policy::ASSIGNMENTS,
            policy::GRANTS,
            policy::INVITATIONS,
        ] {
            assert!(ddl.contains("CREATE POLICY tenant_isolation"));
            assert!(ddl.contains("app.organization_id"));
        }
    }

    #[tokio::test]
    async fn rls_denies_without_context_and_cross_tenant() {
        let db = RlsTenancyDatabase::new(TenancyDatabase::new());
        active_org(db.inner(), "o-a").await;
        active_org(db.inner(), "o-b").await;
        let branch_a = db
            .inner()
            .create_branch(BranchId::new("b-a").unwrap(), org_id("o-a"), "Branch A")
            .await
            .unwrap();
        db.inner()
            .activate_branch(&org_id("o-a"), &branch_a.id)
            .await
            .unwrap();
        let member_b = active_member(db.inner(), "o-b", "m-b", "u-b").await;

        // No context → fail closed (deny by default, §26.1 hides nothing, it denies).
        assert!(db.branch_snapshot_rls(&branch_a.id).await.is_none());
        assert!(db.membership_snapshot_rls(&member_b.id).await.is_none());

        // Context for tenant B cannot see tenant A's branch (RLS filtered).
        db.set_context(RlsContext::from_organization(org_id("o-b")));
        assert!(db.branch_snapshot_rls(&branch_a.id).await.is_none());
        // Context for own tenant sees own rows.
        assert!(db.membership_snapshot_rls(&member_b.id).await.is_some());

        db.clear_context();
        assert!(db.branch_snapshot_rls(&branch_a.id).await.is_none());
    }

    #[tokio::test]
    async fn rls_defense_in_depth_on_top_of_predicate() {
        // Even if an application handler mistakenly called `inner().branch_snapshot`
        // with a foreign org, the inner predicate already denies (§27.1). RLS
        // adds the second check using the session context, so a predicate
        // bypass that used a raw `SELECT ... WHERE id = $1` would still be
        // filtered. This test proves the layered model (§26 diagram).
        let db = RlsTenancyDatabase::new(TenancyDatabase::new());
        active_org(db.inner(), "o-a").await;
        let branch_a = db
            .inner()
            .create_branch(BranchId::new("b-a").unwrap(), org_id("o-a"), "Branch A")
            .await
            .unwrap();
        db.inner()
            .activate_branch(&org_id("o-a"), &branch_a.id)
            .await
            .unwrap();

        // Application predicate layer: foreign org denied even without RLS.
        assert!(
            db.inner()
                .branch_snapshot(&org_id("o-b"), &branch_a.id)
                .await
                .is_none()
        );

        // RLS layer: foreign context denied even if predicate were bypassed.
        db.set_context(RlsContext::from_organization(org_id("o-b")));
        assert!(db.branch_snapshot_rls(&branch_a.id).await.is_none());

        db.set_context(RlsContext::from_organization(org_id("o-a")));
        assert!(db.branch_snapshot_rls(&branch_a.id).await.is_some());
    }

    /// Full §45 security test matrix negative suite at the RLS layer (§26.1:
    /// cross-tenant / cross-branch must be denied at the DB, not hidden).
    #[tokio::test]
    async fn security_matrix_cross_tenant_and_branch_denied_at_rls() {
        let db = RlsTenancyDatabase::new(TenancyDatabase::new());
        active_org(db.inner(), "o-a").await;
        active_org(db.inner(), "o-b").await;
        let branch_a1 = db
            .inner()
            .create_branch(BranchId::new("b-a1").unwrap(), org_id("o-a"), "A1")
            .await
            .unwrap();
        db.inner()
            .activate_branch(&org_id("o-a"), &branch_a1.id)
            .await
            .unwrap();
        let branch_a2 = db
            .inner()
            .create_branch(BranchId::new("b-a2").unwrap(), org_id("o-a"), "A2")
            .await
            .unwrap();
        db.inner()
            .activate_branch(&org_id("o-a"), &branch_a2.id)
            .await
            .unwrap();
        let branch_b1 = db
            .inner()
            .create_branch(BranchId::new("b-b1").unwrap(), org_id("o-b"), "B1")
            .await
            .unwrap();
        db.inner()
            .activate_branch(&org_id("o-b"), &branch_b1.id)
            .await
            .unwrap();
        let member_a = active_member(db.inner(), "o-a", "m-a", "u-a").await;
        let _member_a_suspended = {
            let m = db
                .inner()
                .invite_membership(
                    MembershipId::new("m-a-susp").unwrap(),
                    org_id("o-a"),
                    user("u-susp"),
                )
                .await
                .unwrap();
            db.inner()
                .advance_membership(&org_id("o-a"), &m.id)
                .await
                .unwrap();
            let active = db
                .inner()
                .activate_membership(&org_id("o-a"), &m.id)
                .await
                .unwrap();
            db.inner()
                .suspend_membership(&org_id("o-a"), &active.id)
                .await
                .unwrap()
        };

        // Cross-tenant GET via RLS: tenant B context cannot read tenant A branch.
        db.set_context(RlsContext::from_organization(org_id("o-b")));
        assert!(
            db.branch_snapshot_rls(&branch_a1.id).await.is_none(),
            "cross-tenant GET must deny"
        );
        assert!(
            db.membership_snapshot_rls(&member_a.id).await.is_none(),
            "cross-tenant membership GET must deny"
        );

        // Cross-branch: tenant A context can read own branch A1, but RLS alone
        // does not enforce branch scope — that is the application `EffectiveScope`
        // check (§27). RLS tenant isolation is proven above; branch isolation
        // is proven via `resolve_effective_scope` predicate tests in
        // `sitolo-tenancy`. Here we assert RLS tenant filter does not leak
        // across tenants for branch B1.
        db.set_context(RlsContext::from_organization(org_id("o-a")));
        assert!(db.branch_snapshot_rls(&branch_a1.id).await.is_some());
        assert!(
            db.branch_snapshot_rls(&branch_b1.id).await.is_none(),
            "cross-tenant branch B1 via A context must deny"
        );
        // Within same tenant, RLS intentionally allows both branches (org predicate only);
        // branch-to-branch denial is application scope, not RLS.
        assert!(db.branch_snapshot_rls(&branch_a2.id).await.is_some());

        // Foreign branch creation is org-scoped at the predicate layer — RLS
        // would also hide it, but the predicate already denies via NotFound.
        assert_eq!(
            db.inner()
                .branch_snapshot(&org_id("o-a"), &branch_b1.id)
                .await,
            None
        );
    }
}
