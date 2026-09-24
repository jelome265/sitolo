-- Phase 4 Part 7 / PR-008 & Part 8 / PR-009 Test Fixture Schema & RLS Policies

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

-- 4. IAM Audit Records Table
CREATE TABLE IF NOT EXISTS iam_audit_records (
    event_id VARCHAR(128) PRIMARY KEY,
    event_name VARCHAR(128) NOT NULL,
    event_version INT NOT NULL DEFAULT 1,
    occurred_at TIMESTAMPTZ NOT NULL,
    organization_id VARCHAR(128),
    branch_id VARCHAR(128),
    actor_subject_ref VARCHAR(128),
    actor_membership_ref VARCHAR(128),
    actor_device_ref VARCHAR(128),
    request_id VARCHAR(128),
    trace_id VARCHAR(128),
    target_type VARCHAR(128),
    target_ref VARCHAR(128),
    action VARCHAR(256) NOT NULL,
    result VARCHAR(32) NOT NULL,
    reason_class VARCHAR(128),
    assurance_level VARCHAR(32),
    source VARCHAR(64) NOT NULL,
    metadata TEXT
);

-- 5. Outbox Events Table
CREATE TABLE IF NOT EXISTS outbox_events (
    event_id VARCHAR(128) PRIMARY KEY,
    aggregate_type VARCHAR(128) NOT NULL,
    aggregate_id VARCHAR(128) NOT NULL,
    event_name VARCHAR(256) NOT NULL,
    event_version INT NOT NULL DEFAULT 1,
    organization_id VARCHAR(128),
    branch_id VARCHAR(128),
    occurred_at TIMESTAMPTZ NOT NULL,
    payload TEXT NOT NULL,
    status VARCHAR(32) NOT NULL DEFAULT 'PENDING',
    available_at TIMESTAMPTZ NOT NULL,
    attempt_count INT NOT NULL DEFAULT 0,
    locked_at TIMESTAMPTZ,
    published_at TIMESTAMPTZ,
    last_error_class VARCHAR(256),
    deduplication_key VARCHAR(256) NOT NULL UNIQUE,
    schema_version INT NOT NULL DEFAULT 1
);

-- 6. Enable and Force Row Level Security (RLS)
ALTER TABLE organizations ENABLE ROW LEVEL SECURITY;
ALTER TABLE organizations FORCE ROW LEVEL SECURITY;

ALTER TABLE branches ENABLE ROW LEVEL SECURITY;
ALTER TABLE branches FORCE ROW LEVEL SECURITY;

ALTER TABLE tenant_resources ENABLE ROW LEVEL SECURITY;
ALTER TABLE tenant_resources FORCE ROW LEVEL SECURITY;

ALTER TABLE iam_audit_records ENABLE ROW LEVEL SECURITY;
ALTER TABLE iam_audit_records FORCE ROW LEVEL SECURITY;

ALTER TABLE outbox_events ENABLE ROW LEVEL SECURITY;
ALTER TABLE outbox_events FORCE ROW LEVEL SECURITY;

-- 7. Define RLS Policies for app_runtime

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

-- IAM Audit Records Policy
DROP POLICY IF EXISTS iam_audit_records_isolation_policy ON iam_audit_records;
CREATE POLICY iam_audit_records_isolation_policy ON iam_audit_records
    FOR ALL
    TO app_runtime
    USING (
        organization_id IS NULL
        OR organization_id = NULLIF(current_setting('app.organization_id', true), '')
    )
    WITH CHECK (
        organization_id IS NULL
        OR organization_id = NULLIF(current_setting('app.organization_id', true), '')
    );

-- Outbox Events Policy
DROP POLICY IF EXISTS outbox_events_isolation_policy ON outbox_events;
CREATE POLICY outbox_events_isolation_policy ON outbox_events
    FOR ALL
    TO app_runtime
    USING (
        true
    )
    WITH CHECK (
        organization_id IS NULL
        OR organization_id = NULLIF(current_setting('app.organization_id', true), '')
    );
