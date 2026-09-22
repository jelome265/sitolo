-- Phase 4 Part 7 / PR-008 Test Fixture Schema & RLS Policies

-- 1. Organizations Table
CREATE TABLE IF NOT EXISTS organizations (
    id VARCHAR(128) PRIMARY KEY,
    name VARCHAR(256) NOT NULL,
    state VARCHAR(32) NOT NULL,
    state_version BIGINT NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 2. Branches Table
CREATE TABLE IF NOT EXISTS branches (
    id VARCHAR(128) PRIMARY KEY,
    organization_id VARCHAR(128) NOT NULL REFERENCES organizations(id) ON DELETE RESTRICT,
    name VARCHAR(256) NOT NULL,
    state VARCHAR(32) NOT NULL,
    state_version BIGINT NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT branches_id_org_unique UNIQUE (id, organization_id)
);

-- 3. Tenant-Owned Representative Resources Table
CREATE TABLE IF NOT EXISTS tenant_resources (
    id VARCHAR(128) PRIMARY KEY,
    organization_id VARCHAR(128) NOT NULL REFERENCES organizations(id) ON DELETE RESTRICT,
    branch_id VARCHAR(128) NOT NULL,
    data TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT fk_tenant_resources_branch FOREIGN KEY (branch_id, organization_id) REFERENCES branches(id, organization_id) ON DELETE RESTRICT
);

-- Grant table privileges to runtime role
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO app_runtime;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO app_runtime;

-- 4. Enable and Force Row Level Security (RLS)
ALTER TABLE organizations ENABLE ROW LEVEL SECURITY;
ALTER TABLE organizations FORCE ROW LEVEL SECURITY;

ALTER TABLE branches ENABLE ROW LEVEL SECURITY;
ALTER TABLE branches FORCE ROW LEVEL SECURITY;

ALTER TABLE tenant_resources ENABLE ROW LEVEL SECURITY;
ALTER TABLE tenant_resources FORCE ROW LEVEL SECURITY;

-- 5. Define RLS Policies for app_runtime

-- Organizations Policy
DROP POLICY IF EXISTS organizations_isolation_policy ON organizations;
CREATE POLICY organizations_isolation_policy ON organizations
    FOR ALL
    TO app_runtime
    USING (
        id = NULLIF(current_setting('app.organization_id', true), '')
    )
    WITH CHECK (
        id = NULLIF(current_setting('app.organization_id', true), '')
    );

-- Branches Policy
DROP POLICY IF EXISTS branches_isolation_policy ON branches;
CREATE POLICY branches_isolation_policy ON branches
    FOR ALL
    TO app_runtime
    USING (
        organization_id = NULLIF(current_setting('app.organization_id', true), '')
        AND (
            NULLIF(current_setting('app.branch_id', true), '') IS NULL
            OR id = current_setting('app.branch_id', true)
        )
    )
    WITH CHECK (
        organization_id = NULLIF(current_setting('app.organization_id', true), '')
        AND (
            NULLIF(current_setting('app.branch_id', true), '') IS NULL
            OR id = current_setting('app.branch_id', true)
        )
    );

-- Tenant Resources Policy
DROP POLICY IF EXISTS tenant_resources_isolation_policy ON tenant_resources;
CREATE POLICY tenant_resources_isolation_policy ON tenant_resources
    FOR ALL
    TO app_runtime
    USING (
        organization_id = NULLIF(current_setting('app.organization_id', true), '')
        AND (
            NULLIF(current_setting('app.branch_id', true), '') IS NULL
            OR branch_id = current_setting('app.branch_id', true)
        )
    )
    WITH CHECK (
        organization_id = NULLIF(current_setting('app.organization_id', true), '')
        AND (
            NULLIF(current_setting('app.branch_id', true), '') IS NULL
            OR branch_id = current_setting('app.branch_id', true)
        )
    );
