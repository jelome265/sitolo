-- Additive transactional inbox for the approved SNS FIFO -> SQS FIFO relay.
-- Apply only through the externally approved production migration/release
-- process. The consumer effect and this row must share one SQL transaction.

CREATE TABLE consumer_inbox (
    consumer_name TEXT NOT NULL CHECK (consumer_name ~ '^[a-z][a-z0-9_-]{0,63}$'),
    event_id TEXT NOT NULL CHECK (length(event_id) BETWEEN 1 AND 128),
    event_name TEXT NOT NULL CHECK (length(event_name) BETWEEN 1 AND 128),
    event_version INTEGER NOT NULL CHECK (event_version > 0),
    organization_id TEXT NOT NULL CHECK (length(organization_id) BETWEEN 1 AND 128),
    branch_id TEXT CHECK (branch_id IS NULL OR length(branch_id) BETWEEN 1 AND 128),
    aggregate_type TEXT NOT NULL CHECK (length(aggregate_type) BETWEEN 1 AND 128),
    aggregate_id TEXT NOT NULL CHECK (length(aggregate_id) BETWEEN 1 AND 128),
    aggregate_sequence BIGINT NOT NULL CHECK (aggregate_sequence > 0),
    occurred_at TIMESTAMPTZ NOT NULL,
    payload_sha256 BYTEA NOT NULL CHECK (octet_length(payload_sha256) = 32),
    committed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (consumer_name, event_id),
    UNIQUE (
        consumer_name,
        organization_id,
        aggregate_type,
        aggregate_id,
        aggregate_sequence
    )
);

ALTER TABLE consumer_inbox ENABLE ROW LEVEL SECURITY;
ALTER TABLE consumer_inbox FORCE ROW LEVEL SECURITY;

CREATE POLICY consumer_inbox_tenant_policy ON consumer_inbox
    FOR ALL TO app_worker
    USING (
        organization_id = current_setting('app.organization_id', true)
        AND current_setting('app.organization_id', true) <> ''
    )
    WITH CHECK (
        organization_id = current_setting('app.organization_id', true)
        AND current_setting('app.organization_id', true) <> ''
    );

GRANT SELECT, INSERT ON consumer_inbox TO app_worker;
