-- Phase 4 Part 8: durable organization provisioning tables.
-- Apply with the database migration/administrative identity after the
-- Part 8 app_runtime and app_outbox_owner roles exist. The API runtime role
-- receives SELECT and INSERT plus narrowly scoped UPDATE of membership state
-- and state_version. RLS tenant predicates apply to every operation.

CREATE TABLE organizations (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL CHECK (length(btrim(name)) BETWEEN 1 AND 256),
    state TEXT NOT NULL CHECK (state IN ('PROVISIONING', 'ACTIVE', 'SUSPENDED', 'CLOSING', 'CLOSED')),
    state_version BIGINT NOT NULL CHECK (state_version > 0)
);

CREATE TABLE organization_memberships (
    id TEXT PRIMARY KEY,
    organization_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE RESTRICT,
    user_id TEXT NOT NULL,
    state TEXT NOT NULL CHECK (state IN ('INVITED', 'PENDING_ACCEPTANCE', 'EXPIRED', 'ACTIVE', 'SUSPENDED', 'REVOKED')),
    state_version BIGINT NOT NULL CHECK (state_version > 0),
    UNIQUE (organization_id, id)
);

CREATE UNIQUE INDEX uq_membership_org_user_nonterminal
    ON organization_memberships (organization_id, user_id)
    WHERE state NOT IN ('REVOKED', 'EXPIRED');

CREATE TABLE role_assignments (
    id TEXT PRIMARY KEY,
    organization_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE RESTRICT,
    membership_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK (role IN (
        'OWNER', 'ORG_ADMIN', 'BRANCH_MANAGER', 'CASHIER', 'INVENTORY_CLERK',
        'PROCUREMENT_CLERK', 'ACCOUNTING_USER', 'VIEWER', 'AUDITOR'
    )),
    state TEXT NOT NULL CHECK (state IN (
        'REQUESTED', 'VALIDATING', 'REJECTED', 'APPROVAL_REQUIRED',
        'APPROVED', 'EFFECTIVE', 'REVOKED'
    )),
    state_version BIGINT NOT NULL CHECK (state_version > 0),
    UNIQUE (organization_id, id),
    FOREIGN KEY (organization_id, membership_id)
        REFERENCES organization_memberships(organization_id, id) ON DELETE RESTRICT
);

CREATE UNIQUE INDEX uq_role_assignment_effective
    ON role_assignments (organization_id, membership_id, role)
    WHERE state = 'EFFECTIVE';

CREATE TABLE branches (
    id TEXT PRIMARY KEY,
    organization_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE RESTRICT,
    name TEXT NOT NULL CHECK (length(btrim(name)) BETWEEN 1 AND 256),
    state TEXT NOT NULL CHECK (state IN ('PROVISIONING', 'ACTIVE', 'SUSPENDED', 'CLOSING', 'CLOSED')),
    state_version BIGINT NOT NULL CHECK (state_version > 0),
    UNIQUE (organization_id, id)
);

CREATE TABLE invitations (
    id TEXT PRIMARY KEY,
    organization_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE RESTRICT,
    membership_id TEXT NOT NULL,
    proposed_role TEXT NOT NULL CHECK (proposed_role IN (
        'OWNER', 'ORG_ADMIN', 'BRANCH_MANAGER', 'CASHIER', 'INVENTORY_CLERK',
        'PROCUREMENT_CLERK', 'ACCOUNTING_USER', 'VIEWER', 'AUDITOR'
    )),
    scope_kind TEXT CHECK (scope_kind IN ('ORGANIZATION', 'BRANCH')),
    branch_id TEXT,
    contact_kind TEXT CHECK (contact_kind IN ('EMAIL', 'PHONE')),
    contact_value TEXT,
    token_hash BYTEA UNIQUE,
    state TEXT NOT NULL CHECK (state IN ('ISSUED', 'ACCEPTED', 'WITHDRAWN', 'EXPIRED')),
    issued_at TIMESTAMPTZ NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    state_version BIGINT NOT NULL CHECK (state_version > 0),
    CHECK ((scope_kind = 'BRANCH') = (branch_id IS NOT NULL)),
    CHECK ((contact_kind IS NULL) = (contact_value IS NULL)),
    CHECK (token_hash IS NULL OR octet_length(token_hash) = 32),
    CHECK (
        (state = 'ISSUED' AND contact_kind IS NOT NULL AND token_hash IS NOT NULL)
        OR (state <> 'ISSUED' AND contact_kind IS NULL AND token_hash IS NULL)
    ),
    CHECK (expires_at > issued_at),
    FOREIGN KEY (organization_id, membership_id) REFERENCES organization_memberships(organization_id, id) ON DELETE RESTRICT,
    FOREIGN KEY (organization_id, branch_id) REFERENCES branches(organization_id, id) ON DELETE RESTRICT
);

ALTER TABLE organizations ENABLE ROW LEVEL SECURITY;
ALTER TABLE organizations FORCE ROW LEVEL SECURITY;
ALTER TABLE organization_memberships ENABLE ROW LEVEL SECURITY;
ALTER TABLE organization_memberships FORCE ROW LEVEL SECURITY;
ALTER TABLE branches ENABLE ROW LEVEL SECURITY;
ALTER TABLE branches FORCE ROW LEVEL SECURITY;
ALTER TABLE invitations ENABLE ROW LEVEL SECURITY;
ALTER TABLE invitations FORCE ROW LEVEL SECURITY;
ALTER TABLE role_assignments ENABLE ROW LEVEL SECURITY;
ALTER TABLE role_assignments FORCE ROW LEVEL SECURITY;

CREATE POLICY organizations_provision_insert ON organizations
    FOR INSERT TO app_runtime
    WITH CHECK (
        id = NULLIF(current_setting('app.organization_id', true), '')
        AND current_setting('app.organization_id', true) <> ''
    );
CREATE POLICY organizations_provision_read ON organizations
    FOR SELECT TO app_runtime
    USING (id = NULLIF(current_setting('app.organization_id', true), ''));
CREATE POLICY organizations_transition_read ON organizations
    FOR SELECT TO app_outbox_owner
    USING (
        id = NULLIF(current_setting('app.organization_id', true), '')
        AND current_setting('app.organization_id', true) <> ''
    );
CREATE POLICY organizations_transition_update ON organizations
    FOR UPDATE TO app_outbox_owner
    USING (id = NULLIF(current_setting('app.organization_id', true), ''))
    WITH CHECK (
        id = NULLIF(current_setting('app.organization_id', true), '')
        AND current_setting('app.organization_id', true) <> ''
    );

CREATE POLICY memberships_provision_insert ON organization_memberships
    FOR INSERT TO app_runtime
    WITH CHECK (
        organization_id = NULLIF(current_setting('app.organization_id', true), '')
        AND current_setting('app.organization_id', true) <> ''
    );
CREATE POLICY memberships_provision_read ON organization_memberships
    FOR SELECT TO app_runtime
    USING (organization_id = NULLIF(current_setting('app.organization_id', true), ''));
CREATE POLICY memberships_transition_update ON organization_memberships
    FOR UPDATE TO app_runtime
    USING (organization_id = NULLIF(current_setting('app.organization_id', true), ''))
    WITH CHECK (
        organization_id = NULLIF(current_setting('app.organization_id', true), '')
        AND current_setting('app.organization_id', true) <> ''
    );

CREATE POLICY branches_provision_insert ON branches
    FOR INSERT TO app_runtime
    WITH CHECK (
        organization_id = NULLIF(current_setting('app.organization_id', true), '')
        AND current_setting('app.organization_id', true) <> ''
    );
CREATE POLICY branches_provision_read ON branches
    FOR SELECT TO app_runtime
    USING (organization_id = NULLIF(current_setting('app.organization_id', true), ''));
CREATE POLICY branches_transition_read ON branches
    FOR SELECT TO app_outbox_owner
    USING (
        organization_id = NULLIF(current_setting('app.organization_id', true), '')
        AND current_setting('app.organization_id', true) <> ''
    );
CREATE POLICY branches_transition_update ON branches
    FOR UPDATE TO app_outbox_owner
    USING (organization_id = NULLIF(current_setting('app.organization_id', true), ''))
    WITH CHECK (
        organization_id = NULLIF(current_setting('app.organization_id', true), '')
        AND current_setting('app.organization_id', true) <> ''
    );
CREATE POLICY invitations_tenant_insert ON invitations
    FOR INSERT TO app_runtime
    WITH CHECK (
        organization_id = NULLIF(current_setting('app.organization_id', true), '')
        AND current_setting('app.organization_id', true) <> ''
    );
CREATE POLICY invitations_tenant_read ON invitations
    FOR SELECT TO app_runtime
    USING (organization_id = NULLIF(current_setting('app.organization_id', true), ''));
CREATE POLICY invitations_terminal_update ON invitations
    FOR UPDATE TO app_runtime
    USING (organization_id = NULLIF(current_setting('app.organization_id', true), ''))
    WITH CHECK (
        organization_id = NULLIF(current_setting('app.organization_id', true), '')
        AND current_setting('app.organization_id', true) <> ''
    );
CREATE POLICY role_assignments_tenant_all ON role_assignments
    FOR ALL TO app_runtime
    USING (
        organization_id = NULLIF(current_setting('app.organization_id', true), '')
    )
    WITH CHECK (
        organization_id = NULLIF(current_setting('app.organization_id', true), '')
        AND current_setting('app.organization_id', true) <> ''
    );

ALTER TABLE organizations OWNER TO app_outbox_owner;
ALTER TABLE organization_memberships OWNER TO app_outbox_owner;
ALTER TABLE branches OWNER TO app_outbox_owner;
ALTER TABLE invitations OWNER TO app_outbox_owner;
ALTER TABLE role_assignments OWNER TO app_outbox_owner;

-- SECURITY DEFINER transition functions keep direct UPDATE unavailable to
-- app_runtime. Their owner remains subject to forced RLS and can only update
-- rows matching the transaction-local tenant context.
-- Serialize organization-scoped mutations without granting app_runtime row
-- UPDATE privileges (which PostgreSQL requires for SELECT ... FOR UPDATE).
CREATE FUNCTION lock_organization_mutations(p_organization_id TEXT)
RETURNS TEXT
LANGUAGE plpgsql
SECURITY INVOKER
SET search_path FROM CURRENT
AS $$
DECLARE
    organization_state TEXT;
BEGIN
    IF p_organization_id IS NULL OR p_organization_id = '' OR
       p_organization_id <> NULLIF(current_setting('app.organization_id', true), '') THEN
        RAISE EXCEPTION 'organization does not match tenant context';
    END IF;

    PERFORM pg_catalog.pg_advisory_xact_lock(
        pg_catalog.hashtextextended(p_organization_id, 0)
    );
    SELECT state INTO organization_state
    FROM organizations
    WHERE id = p_organization_id;
    RETURN organization_state;
END;
$$;

CREATE FUNCTION transition_organization_state(p_from TEXT, p_to TEXT)
RETURNS TABLE(name TEXT, state_version BIGINT)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path FROM CURRENT
AS $$
BEGIN
    PERFORM pg_catalog.pg_advisory_xact_lock(
        pg_catalog.hashtextextended(
            NULLIF(current_setting('app.organization_id', true), ''), 0
        )
    );

    IF NOT (
        (p_from = 'PROVISIONING' AND p_to = 'ACTIVE') OR
        (p_from = 'ACTIVE' AND p_to IN ('SUSPENDED', 'CLOSING')) OR
        (p_from = 'SUSPENDED' AND p_to = 'ACTIVE') OR
        (p_from = 'CLOSING' AND p_to = 'CLOSED')
    ) THEN
        RAISE EXCEPTION 'invalid organization transition';
    END IF;

    RETURN QUERY
    UPDATE organizations AS organization
    SET state = p_to, state_version = organization.state_version + 1
    WHERE organization.id = NULLIF(current_setting('app.organization_id', true), '')
      AND organization.state = p_from
    RETURNING organization.name, organization.state_version;
END;
$$;

CREATE FUNCTION transition_branch_state(p_branch_id TEXT, p_from TEXT, p_to TEXT)
RETURNS TABLE(name TEXT, state_version BIGINT)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path FROM CURRENT
AS $$
BEGIN
    PERFORM pg_catalog.pg_advisory_xact_lock(
        pg_catalog.hashtextextended(
            NULLIF(current_setting('app.organization_id', true), ''), 0
        )
    );

    IF NOT (
        (p_from = 'PROVISIONING' AND p_to = 'ACTIVE') OR
        (p_from = 'ACTIVE' AND p_to IN ('SUSPENDED', 'CLOSING')) OR
        (p_from = 'SUSPENDED' AND p_to = 'ACTIVE') OR
        (p_from = 'CLOSING' AND p_to = 'CLOSED')
    ) THEN
        RAISE EXCEPTION 'invalid branch transition';
    END IF;

    RETURN QUERY
    UPDATE branches AS branch
    SET state = p_to, state_version = branch.state_version + 1
    WHERE branch.organization_id = NULLIF(current_setting('app.organization_id', true), '')
      AND branch.id = p_branch_id
      AND branch.state = p_from
    RETURNING branch.name, branch.state_version;
END;
$$;

ALTER FUNCTION transition_organization_state(TEXT, TEXT) OWNER TO app_outbox_owner;
ALTER FUNCTION transition_branch_state(TEXT, TEXT, TEXT) OWNER TO app_outbox_owner;
REVOKE ALL ON FUNCTION lock_organization_mutations(TEXT) FROM PUBLIC;
REVOKE ALL ON FUNCTION transition_organization_state(TEXT, TEXT) FROM PUBLIC;
REVOKE ALL ON FUNCTION transition_branch_state(TEXT, TEXT, TEXT) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION lock_organization_mutations(TEXT) TO app_runtime;
GRANT EXECUTE ON FUNCTION transition_organization_state(TEXT, TEXT) TO app_runtime;
GRANT EXECUTE ON FUNCTION transition_branch_state(TEXT, TEXT, TEXT) TO app_runtime;

REVOKE ALL ON organizations, organization_memberships, branches, invitations, role_assignments FROM PUBLIC;
GRANT SELECT, INSERT ON organizations, branches TO app_runtime;
GRANT SELECT, INSERT ON invitations TO app_runtime;
GRANT UPDATE (state, state_version, contact_kind, contact_value, token_hash) ON invitations TO app_runtime;
GRANT SELECT, INSERT ON role_assignments TO app_runtime;
GRANT UPDATE (state, state_version) ON role_assignments TO app_runtime;
GRANT SELECT, INSERT ON organization_memberships TO app_runtime;
GRANT UPDATE (state, state_version) ON organization_memberships TO app_runtime;
