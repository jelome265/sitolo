-- Phase 4 Part 8 audit/outbox schema fixture for TenancyService evidence
-- integration tests. Mirrors
-- crates/sitolo-persistence/tests/fixtures/part8_audit_outbox_schema.sql,
-- trimmed to the two tables these tests exercise directly through an
-- admin/superuser pool (RLS/least-privilege behavior is already covered by
-- crates/sitolo-persistence/tests/rls_security_tests.rs and does not need
-- re-proving here).

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
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_audit_event_name CHECK (event_name ~ '^[a-z][a-z0-9]*(\.[a-z][a-z0-9]*)*$'),
    CONSTRAINT chk_audit_result CHECK (result IN ('SUCCESS', 'FAILURE'))
);

CREATE INDEX IF NOT EXISTS idx_audit_events_org ON audit_events(organization_id);

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
    aggregate_sequence BIGINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_outbox_status CHECK (status IN ('PENDING', 'CLAIMED', 'PUBLISHED', 'QUARANTINED')),
    CONSTRAINT chk_outbox_event_name CHECK (event_name ~ '^[a-z][a-z0-9]*(\.[a-z][a-z0-9]*)*$')
);

CREATE INDEX IF NOT EXISTS idx_outbox_events_org ON outbox_events(organization_id);
CREATE INDEX IF NOT EXISTS idx_outbox_events_aggregate ON outbox_events(aggregate_type, aggregate_id, aggregate_sequence);
