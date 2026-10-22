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
    UNIQUE (organization_id, user_id)
);

CREATE TABLE branches (
    id TEXT PRIMARY KEY,
    organization_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE RESTRICT,
    name TEXT NOT NULL CHECK (length(btrim(name)) BETWEEN 1 AND 256),
    state TEXT NOT NULL CHECK (state IN ('PROVISIONING', 'ACTIVE', 'SUSPENDED', 'CLOSING', 'CLOSED')),
    state_version BIGINT NOT NULL CHECK (state_version > 0),
    UNIQUE (organization_id, id)
);

ALTER TABLE organizations ENABLE ROW LEVEL SECURITY;
ALTER TABLE organizations FORCE ROW LEVEL SECURITY;
ALTER TABLE organization_memberships ENABLE ROW LEVEL SECURITY;
ALTER TABLE organization_memberships FORCE ROW LEVEL SECURITY;
ALTER TABLE branches ENABLE ROW LEVEL SECURITY;
ALTER TABLE branches FORCE ROW LEVEL SECURITY;

CREATE POLICY organizations_provision_insert ON organizations
    FOR INSERT TO app_runtime
    WITH CHECK (
        id = NULLIF(current_setting('app.organization_id', true), '')
        AND current_setting('app.organization_id', true) <> ''
    );
CREATE POLICY organizations_provision_read ON organizations
    FOR SELECT TO app_runtime
    USING (id = NULLIF(current_setting('app.organization_id', true), ''));

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

ALTER TABLE organizations OWNER TO app_outbox_owner;
ALTER TABLE organization_memberships OWNER TO app_outbox_owner;
ALTER TABLE branches OWNER TO app_outbox_owner;

REVOKE ALL ON organizations, organization_memberships, branches FROM PUBLIC;
GRANT SELECT, INSERT ON organizations, branches TO app_runtime;
GRANT SELECT, INSERT ON organization_memberships TO app_runtime;
GRANT UPDATE (state, state_version) ON organization_memberships TO app_runtime;
