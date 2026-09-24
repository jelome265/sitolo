-- Phase 4 Part 8 Audit and Outbox Schema

CREATE TABLE IF NOT EXISTS audit_events (
    event_id TEXT PRIMARY KEY,
    event_name TEXT NOT NULL,
    event_version INTEGER NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    organization_id TEXT NOT NULL,
    branch_id TEXT,
    actor_subject_ref TEXT,
    actor_membership_ref TEXT,
    actor_device_ref TEXT,
    request_id TEXT,
    trace_id TEXT,
    target_type TEXT NOT NULL,
    target_ref TEXT NOT NULL,
    action TEXT NOT NULL,
    result TEXT NOT NULL,
    reason_class TEXT,
    assurance_level TEXT,
    source TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_audit_events_org ON audit_events(organization_id);
CREATE INDEX IF NOT EXISTS idx_audit_events_occurred ON audit_events(occurred_at);

ALTER TABLE audit_events ENABLE ROW LEVEL SECURITY;
ALTER TABLE audit_events FORCE ROW LEVEL SECURITY;

CREATE TABLE IF NOT EXISTS outbox_events (
    event_id TEXT PRIMARY KEY,
    aggregate_type TEXT NOT NULL,
    aggregate_id TEXT NOT NULL,
    event_name TEXT NOT NULL,
    event_version INTEGER NOT NULL,
    organization_id TEXT NOT NULL,
    branch_id TEXT,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    payload JSONB NOT NULL,
    status TEXT NOT NULL DEFAULT 'PENDING',
    available_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    attempt_count INTEGER NOT NULL DEFAULT 0,
    locked_at TIMESTAMPTZ,
    published_at TIMESTAMPTZ,
    last_error_class TEXT,
    deduplication_key TEXT NOT NULL UNIQUE,
    schema_version INTEGER NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_outbox_events_status ON outbox_events(status, available_at);
CREATE INDEX IF NOT EXISTS idx_outbox_events_org ON outbox_events(organization_id);

ALTER TABLE outbox_events ENABLE ROW LEVEL SECURITY;
ALTER TABLE outbox_events FORCE ROW LEVEL SECURITY;

GRANT INSERT ON audit_events TO app_runtime;
GRANT INSERT ON outbox_events TO app_runtime;

CREATE POLICY audit_events_insert_policy ON audit_events
    FOR INSERT
    TO app_runtime
    WITH CHECK (organization_id = current_setting('app.organization_id', true));

CREATE POLICY outbox_events_insert_policy ON outbox_events
    FOR INSERT
    TO app_runtime
    WITH CHECK (organization_id = current_setting('app.organization_id', true));

DO $$
BEGIN
    IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'app_worker') THEN
        CREATE ROLE app_worker NOLOGIN;
    END IF;
END
$$;

GRANT SELECT, UPDATE ON outbox_events TO app_worker;

CREATE POLICY outbox_events_worker_select_policy ON outbox_events
    FOR SELECT
    TO app_worker
    USING (true);

CREATE POLICY outbox_events_worker_update_policy ON outbox_events
    FOR UPDATE
    TO app_worker
    USING (true)
    WITH CHECK (true);

ALTER TABLE audit_events ADD CONSTRAINT chk_audit_event_name
    CHECK (event_name ~ '^[a-z][a-z0-9]*(\.[a-z][a-z0-9]*)*$');

ALTER TABLE outbox_events ADD CONSTRAINT chk_outbox_status
    CHECK (status IN ('PENDING', 'CLAIMED', 'PUBLISHED', 'QUARANTINED'));

ALTER TABLE audit_events ADD CONSTRAINT chk_audit_result
    CHECK (result IN ('SUCCESS', 'FAILURE'));
