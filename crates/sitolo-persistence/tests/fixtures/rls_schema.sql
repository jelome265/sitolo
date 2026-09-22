-- Phase 4 Part 7 / PR-008 Test Fixture Schema & RLS Policies

-- 1. Create runtime role if not exists
DO $$
BEGIN
    IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'app_runtime') THEN
        CREATE ROLE app_runtime WITH LOGIN NOSUPERUSER NOINHERIT NOCREATEDB NOCREATEROLE NOBYPASSRLS;
    END IF;
END
$$;

-- Grant schema privileges on search_path schema
GRANT USAGE ON SCHEMA public TO app_runtime;

-- 2. Organizations Table
CREATE TABLE IF NOT EXISTS organizations (
    id VARCHAR(128) PRIMARY KEY,
    name VARCHAR(256) NOT NULL,
    state VARCHAR(32) NOT NULL,
    state_version BIGINT NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 3. Branches Table
CREATE TABLE IF NOT EXISTS branches (
    id VARCHAR(128) PRIMARY KEY,
    organization_id VARCHAR(128) NOT NULL REFERENCES organizations(id) ON DELETE RESTRICT,
    name VARCHAR(256) NOT NULL,
    state VARCHAR(32) NOT NULL,
    state_version BIGINT NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT branches_id_org_unique UNIQUE (id, organization_id)
);

-- 4. Tenant-Owned Representative Resources Table
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

-- 5. Enable and Force Row Level Security (RLS)
ALTER TABLE organizations ENABLE ROW LEVEL SECURITY;
ALTER TABLE organizations FORCE ROW LEVEL SECURITY;

ALTER TABLE branches ENABLE ROW LEVEL SECURITY;
ALTER TABLE branches FORCE ROW LEVEL SECURITY;

ALTER TABLE tenant_resources ENABLE ROW LEVEL SECURITY;
ALTER TABLE tenant_resources FORCE ROW LEVEL SECURITY;

-- 6. Define RLS Policies for app_runtime

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
