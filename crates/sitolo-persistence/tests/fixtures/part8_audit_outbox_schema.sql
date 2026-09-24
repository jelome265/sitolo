-- Phase 4 Part 8 Audit and Outbox Schema
-- Real PostgreSQL security boundary with least-privilege worker authority

-- ============================================================================
-- AUDIT EVENTS TABLE
-- ============================================================================

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
CREATE INDEX IF NOT EXISTS idx_audit_events_occurred ON audit_events(occurred_at);

-- Enable and FORCE RLS
ALTER TABLE audit_events ENABLE ROW LEVEL SECURITY;
ALTER TABLE audit_events FORCE ROW LEVEL SECURITY;

-- Runtime role: INSERT only, scoped to transaction tenant context
GRANT INSERT ON audit_events TO app_runtime;

CREATE POLICY audit_events_insert_policy ON audit_events
    FOR INSERT
    TO app_runtime
    WITH CHECK (organization_id = current_setting('app.organization_id', true));

-- ============================================================================
-- OUTBOX EVENTS TABLE
-- ============================================================================

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

CREATE INDEX IF NOT EXISTS idx_outbox_events_status ON outbox_events(status, available_at);
CREATE INDEX IF NOT EXISTS idx_outbox_events_org ON outbox_events(organization_id);
CREATE INDEX IF NOT EXISTS idx_outbox_events_aggregate ON outbox_events(aggregate_type, aggregate_id, aggregate_sequence);

-- Enable and FORCE RLS
ALTER TABLE outbox_events ENABLE ROW LEVEL SECURITY;
ALTER TABLE outbox_events FORCE ROW LEVEL SECURITY;

-- Runtime role: INSERT only, scoped to transaction tenant context
GRANT INSERT ON outbox_events TO app_runtime;

CREATE POLICY outbox_events_insert_policy ON outbox_events
    FOR INSERT
    TO app_runtime
    WITH CHECK (organization_id = current_setting('app.organization_id', true));

-- ============================================================================
-- WORKER ROLE WITH NARROW AUTHORITY
-- ============================================================================

DO $$
BEGIN
    IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'app_worker') THEN
        CREATE ROLE app_worker NOLOGIN;
    END IF;
END
$$;

-- Worker can SELECT (to claim) and execute narrow state-transition functions
GRANT SELECT ON outbox_events TO app_worker;

-- Narrow worker SELECT policy: can only see events for processing
CREATE POLICY outbox_events_worker_select_policy ON outbox_events
    FOR SELECT
    TO app_worker
    USING (status IN ('PENDING', 'CLAIMED'));

-- ============================================================================
-- NARROW WORKER STATE TRANSITION FUNCTIONS
-- ============================================================================

-- Claim events: bounded batch with row locks and SKIP LOCKED
CREATE OR REPLACE FUNCTION outbox_claim_events(
    p_batch_size INTEGER,
    p_lease_duration INTERVAL
)
RETURNS SETOF outbox_events
LANGUAGE plpgsql
SECURITY DEFINER
AS $$
DECLARE
    v_now TIMESTAMPTZ := NOW();
BEGIN
    -- Update PENDING events to CLAIMED with lease
    RETURN QUERY
    UPDATE outbox_events
    SET status = 'CLAIMED',
        locked_at = v_now,
        attempt_count = attempt_count + 1
    WHERE event_id IN (
        SELECT event_id
        FROM outbox_events
        WHERE status = 'PENDING'
          AND available_at <= v_now
        ORDER BY available_at ASC
        LIMIT p_batch_size
        FOR UPDATE SKIP LOCKED
    )
    RETURNING outbox_events.*;
END;
$$;

-- Mark event as successfully published
CREATE OR REPLACE FUNCTION outbox_mark_published(p_event_id TEXT)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
AS $$
BEGIN
    UPDATE outbox_events
    SET status = 'PUBLISHED',
        published_at = NOW(),
        locked_at = NULL
    WHERE event_id = p_event_id
      AND status = 'CLAIMED';
    
    IF NOT FOUND THEN
        RAISE EXCEPTION 'Event % not found or not in CLAIMED state', p_event_id;
    END IF;
END;
$$;

-- Release event for retry (retryable failure)
CREATE OR REPLACE FUNCTION outbox_release_for_retry(
    p_event_id TEXT,
    p_error_class TEXT,
    p_backoff_interval INTERVAL
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
AS $$
BEGIN
    UPDATE outbox_events
    SET status = 'PENDING',
        available_at = NOW() + p_backoff_interval,
        locked_at = NULL,
        last_error_class = p_error_class
    WHERE event_id = p_event_id
      AND status = 'CLAIMED';
    
    IF NOT FOUND THEN
        RAISE EXCEPTION 'Event % not found or not in CLAIMED state', p_event_id;
    END IF;
END;
$$;

-- Quarantine event (terminal failure)
CREATE OR REPLACE FUNCTION outbox_quarantine(
    p_event_id TEXT,
    p_error_class TEXT
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
AS $$
BEGIN
    UPDATE outbox_events
    SET status = 'QUARANTINED',
        locked_at = NULL,
        last_error_class = p_error_class
    WHERE event_id = p_event_id
      AND status = 'CLAIMED';
    
    IF NOT FOUND THEN
        RAISE EXCEPTION 'Event % not found or not in CLAIMED state', p_event_id;
    END IF;
END;
$$;

-- Recover stale claims (lease expired)
CREATE OR REPLACE FUNCTION outbox_recover_stale_claims(p_lease_timeout INTERVAL)
RETURNS INTEGER
LANGUAGE plpgsql
SECURITY DEFINER
AS $$
DECLARE
    v_count INTEGER;
BEGIN
    UPDATE outbox_events
    SET status = 'PENDING',
        locked_at = NULL,
        last_error_class = 'lease_expired'
    WHERE status = 'CLAIMED'
      AND locked_at < NOW() - p_lease_timeout;
    
    GET DIAGNOSTICS v_count = ROW_COUNT;
    RETURN v_count;
END;
$$;

-- Grant EXECUTE on worker functions
GRANT EXECUTE ON FUNCTION outbox_claim_events(INTEGER, INTERVAL) TO app_worker;
GRANT EXECUTE ON FUNCTION outbox_mark_published(TEXT) TO app_worker;
GRANT EXECUTE ON FUNCTION outbox_release_for_retry(TEXT, TEXT, INTERVAL) TO app_worker;
GRANT EXECUTE ON FUNCTION outbox_quarantine(TEXT, TEXT) TO app_worker;
GRANT EXECUTE ON FUNCTION outbox_recover_stale_claims(INTERVAL) TO app_worker;

-- ============================================================================
-- OBSERVABILITY VIEWS
-- ============================================================================

CREATE OR REPLACE VIEW outbox_metrics AS
SELECT
    status,
    COUNT(*) as event_count,
    MIN(available_at) as oldest_pending,
    EXTRACT(EPOCH FROM (NOW() - MIN(available_at))) as oldest_pending_age_seconds
FROM outbox_events
WHERE status IN ('PENDING', 'CLAIMED')
GROUP BY status;
