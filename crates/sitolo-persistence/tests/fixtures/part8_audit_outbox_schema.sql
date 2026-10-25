-- Phase 4 Part 8 Audit and Outbox Schema
--
-- Security model:
--   * `app_runtime` (LOGIN) may INSERT audit/outbox rows inside the request
--     transaction and may advance aggregate sequences. RLS constrains inserts
--     to the transaction tenant context. The runtime pool authenticates as
--     this role directly; `SET ROLE` is not the authority mechanism.
--   * `app_worker` (LOGIN) has EXECUTE on five narrow state-transition
--     functions and no direct table write privilege. The fixture grants no
--     direct SELECT on `outbox_events`, so one worker replica cannot read
--     another replica's claim token.
--   * `app_outbox_owner` (NOLOGIN, NOBYPASSRLS) owns the tables, functions,
--     and metrics view. Every SECURITY DEFINER function fixes `search_path`
--     to `pg_catalog` plus this schema and has an explicit owner policy.
--   * PUBLIC receives no table, sequence, or function privilege.
--
-- The harness replaces `__SCHEMA__` with a quoted, per-test schema name.

-- ============================================================================
-- ROLES
-- ============================================================================

-- Roles are cluster-global, so parallel test schemas can race on creation.
-- Serialize creation across test processes; duplicate_object alone cannot
-- catch a concurrent unique-index violation from pg_authid.
DO $$
BEGIN
    PERFORM pg_advisory_xact_lock(hashtext('sitolo.part8.role-bootstrap'), 8);
    BEGIN
        CREATE ROLE app_runtime LOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS;
    EXCEPTION WHEN duplicate_object THEN
        NULL;
    END;
    BEGIN
        CREATE ROLE app_worker LOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS;
    EXCEPTION WHEN duplicate_object THEN
        NULL;
    END;
    BEGIN
        CREATE ROLE app_outbox_owner NOLOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOBYPASSRLS;
    EXCEPTION WHEN duplicate_object THEN
        NULL;
    END;
END
$$;

-- The fixture asserts the security attributes even when the roles already
-- exist. This requires the migration/admin role to own the roles or be a
-- superuser; a failure here is intentional, because a BYPASSRLS worker would
-- invalidate every RLS test.
ALTER ROLE app_runtime LOGIN NOINHERIT NOSUPERUSER NOBYPASSRLS NOCREATEDB NOCREATEROLE NOREPLICATION;
ALTER ROLE app_worker LOGIN NOINHERIT NOSUPERUSER NOBYPASSRLS NOCREATEDB NOCREATEROLE NOREPLICATION;
ALTER ROLE app_outbox_owner NOLOGIN NOINHERIT NOSUPERUSER NOBYPASSRLS NOCREATEDB NOCREATEROLE NOREPLICATION;

GRANT USAGE ON SCHEMA __SCHEMA__ TO app_runtime, app_worker, app_outbox_owner;
GRANT CREATE ON SCHEMA __SCHEMA__ TO app_outbox_owner;

-- ============================================================================
-- AUDIT EVENTS TABLE
-- ============================================================================

CREATE TABLE __SCHEMA__.audit_events (
    event_id TEXT PRIMARY KEY,
    event_name TEXT NOT NULL,
    event_version INTEGER NOT NULL CHECK (event_version > 0),
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
    CONSTRAINT chk_audit_event_name CHECK (event_name ~ '^[a-z][a-z0-9]*(_[a-z0-9]+)*(\.[a-z][a-z0-9]*(_[a-z0-9]+)*)*$'),
    CONSTRAINT chk_audit_result CHECK (result IN ('SUCCESS', 'FAILURE'))
);

CREATE INDEX idx_audit_events_org ON __SCHEMA__.audit_events(organization_id);
CREATE INDEX idx_audit_events_occurred ON __SCHEMA__.audit_events(occurred_at);

ALTER TABLE __SCHEMA__.audit_events ENABLE ROW LEVEL SECURITY;
ALTER TABLE __SCHEMA__.audit_events FORCE ROW LEVEL SECURITY;

CREATE POLICY audit_events_insert_policy ON __SCHEMA__.audit_events
    FOR INSERT
    TO app_runtime
    WITH CHECK (
        organization_id = current_setting('app.organization_id', true)
        AND current_setting('app.organization_id', true) <> ''
    );

-- ============================================================================
-- OUTBOX EVENTS TABLE
-- ============================================================================

CREATE TABLE __SCHEMA__.outbox_events (
    event_id TEXT PRIMARY KEY,
    aggregate_type TEXT NOT NULL,
    aggregate_id TEXT NOT NULL,
    event_name TEXT NOT NULL,
    event_version INTEGER NOT NULL CHECK (event_version > 0),
    organization_id TEXT NOT NULL,
    branch_id TEXT,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    payload JSONB NOT NULL,
    status TEXT NOT NULL DEFAULT 'PENDING',
    available_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    attempt_count INTEGER NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
    locked_at TIMESTAMPTZ,
    published_at TIMESTAMPTZ,
    last_error_class TEXT,
    deduplication_key TEXT NOT NULL UNIQUE,
    schema_version INTEGER NOT NULL CHECK (schema_version > 0),
    aggregate_sequence BIGINT NOT NULL CHECK (aggregate_sequence > 0),
    claim_token TEXT CHECK (claim_token IS NULL OR claim_token ~ '^[A-Za-z0-9-]{1,128}$'),
    claim_generation BIGINT NOT NULL DEFAULT 0 CHECK (claim_generation >= 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_outbox_status CHECK (status IN ('PENDING', 'CLAIMED', 'PUBLISHED', 'QUARANTINED')),
    CONSTRAINT chk_outbox_event_name CHECK (event_name ~ '^[a-z][a-z0-9]*(_[a-z0-9]+)*(\.[a-z][a-z0-9]*(_[a-z0-9]+)*)*$'),
    CONSTRAINT chk_outbox_error_class CHECK (
        last_error_class IS NULL OR last_error_class ~ '^[a-z][a-z0-9_.-]{0,63}$'
    ),
    CONSTRAINT uq_outbox_aggregate_sequence
        UNIQUE (organization_id, aggregate_type, aggregate_id, aggregate_sequence),
    CONSTRAINT chk_outbox_claim_state CHECK (
        (status = 'CLAIMED' AND claim_token IS NOT NULL AND locked_at IS NOT NULL)
        OR (status <> 'CLAIMED' AND claim_token IS NULL)
    ),
    CONSTRAINT chk_outbox_published_state CHECK (
        (status = 'PUBLISHED' AND published_at IS NOT NULL)
        OR (status <> 'PUBLISHED')
    )
);

CREATE INDEX idx_outbox_events_status
    ON __SCHEMA__.outbox_events(status, available_at);
CREATE INDEX idx_outbox_events_org
    ON __SCHEMA__.outbox_events(organization_id);
CREATE INDEX idx_outbox_events_aggregate_order
    ON __SCHEMA__.outbox_events(organization_id, aggregate_type, aggregate_id, aggregate_sequence);

ALTER TABLE __SCHEMA__.outbox_events ENABLE ROW LEVEL SECURITY;
ALTER TABLE __SCHEMA__.outbox_events FORCE ROW LEVEL SECURITY;

CREATE POLICY outbox_events_insert_policy ON __SCHEMA__.outbox_events
    FOR INSERT
    TO app_runtime
    WITH CHECK (
        organization_id = current_setting('app.organization_id', true)
        AND current_setting('app.organization_id', true) <> ''
    );

-- The owner is subject to FORCE RLS. Only the SECURITY DEFINER functions act
-- as this role; the explicit policy documents and constrains that access.
CREATE POLICY outbox_events_owner_policy ON __SCHEMA__.outbox_events
    FOR ALL
    TO app_outbox_owner
    USING (true)
    WITH CHECK (true);

-- ============================================================================
-- AGGREGATE SEQUENCE ASSIGNMENT
-- ============================================================================
--
-- The row lock taken by `ON CONFLICT DO UPDATE` serialises concurrent
-- enqueues for one organization-scoped aggregate. The application role never
-- computes MAX()+1.
-- A rolled-back enqueue rolls back the increment in the same transaction.

CREATE TABLE __SCHEMA__.outbox_aggregate_sequences (
    organization_id TEXT NOT NULL,
    aggregate_type TEXT NOT NULL,
    aggregate_id TEXT NOT NULL,
    next_sequence BIGINT NOT NULL DEFAULT 1 CHECK (next_sequence >= 1),
    PRIMARY KEY (organization_id, aggregate_type, aggregate_id)
);

ALTER TABLE __SCHEMA__.outbox_aggregate_sequences OWNER TO app_outbox_owner;
ALTER TABLE __SCHEMA__.outbox_aggregate_sequences ENABLE ROW LEVEL SECURITY;
ALTER TABLE __SCHEMA__.outbox_aggregate_sequences FORCE ROW LEVEL SECURITY;
CREATE POLICY outbox_aggregate_sequences_tenant_policy
    ON __SCHEMA__.outbox_aggregate_sequences
    FOR ALL TO app_runtime
    USING (
        organization_id = current_setting('app.organization_id', true)
        AND current_setting('app.organization_id', true) <> ''
    )
    WITH CHECK (
        organization_id = current_setting('app.organization_id', true)
        AND current_setting('app.organization_id', true) <> ''
    );
ALTER TABLE __SCHEMA__.audit_events OWNER TO app_outbox_owner;
ALTER TABLE __SCHEMA__.outbox_events OWNER TO app_outbox_owner;

-- ============================================================================
-- TABLE AND SEQUENCE PRIVILEGES
-- ============================================================================

REVOKE ALL ON ALL TABLES IN SCHEMA __SCHEMA__ FROM PUBLIC;
REVOKE ALL ON ALL SEQUENCES IN SCHEMA __SCHEMA__ FROM PUBLIC;

GRANT INSERT ON __SCHEMA__.audit_events TO app_runtime;
GRANT INSERT ON __SCHEMA__.outbox_events TO app_runtime;
GRANT SELECT, INSERT, UPDATE ON __SCHEMA__.outbox_aggregate_sequences TO app_runtime;

-- `app_worker` receives no table privilege. It receives only function EXECUTE
-- below, so a worker cannot read another worker's claim token directly.

-- ============================================================================
-- NARROW WORKER STATE TRANSITION FUNCTIONS
-- ============================================================================

-- Claims a bounded batch, quarantines events that exhausted their attempt
-- budget, and returns the claim token that fences every later transition.
--
-- Ordering is per aggregate: an event is eligible only when every earlier
-- sequence for the same aggregate is PUBLISHED. Unrelated aggregates proceed
-- in parallel, but one aggregate has at most one in-flight event, so delivery
-- order matches the database-assigned sequence. A quarantined predecessor
-- blocks its aggregate until an operator resolves it through the runbook.
CREATE FUNCTION __SCHEMA__.outbox_claim_events(
    p_batch_size INTEGER,
    p_lease_duration INTERVAL,
    p_max_attempts INTEGER
)
RETURNS SETOF __SCHEMA__.outbox_events
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, __SCHEMA__
AS $$
DECLARE
    v_now TIMESTAMPTZ := NOW();
BEGIN
    IF p_batch_size < 1 OR p_batch_size > 1000 THEN
        RAISE EXCEPTION 'invalid outbox batch size';
    END IF;
    IF p_lease_duration <= INTERVAL '0' OR p_lease_duration > INTERVAL '1 hour' THEN
        RAISE EXCEPTION 'invalid outbox lease duration';
    END IF;
    IF p_max_attempts < 1 OR p_max_attempts > 10 THEN
        RAISE EXCEPTION 'invalid outbox maximum attempts';
    END IF;

    UPDATE __SCHEMA__.outbox_events
    SET status = 'QUARANTINED',
        locked_at = NULL,
        claim_token = NULL,
        last_error_class = 'attempt_budget_exhausted'
    WHERE status = 'PENDING'
      AND attempt_count >= p_max_attempts;

    RETURN QUERY
    WITH candidates AS (
        SELECT event.event_id
        FROM __SCHEMA__.outbox_events AS event
        WHERE event.status = 'PENDING'
          AND event.available_at <= v_now
          AND event.attempt_count < p_max_attempts
          AND NOT EXISTS (
              SELECT 1
              FROM __SCHEMA__.outbox_events AS earlier
              WHERE earlier.organization_id = event.organization_id
                AND earlier.aggregate_type = event.aggregate_type
                AND earlier.aggregate_id = event.aggregate_id
                AND earlier.aggregate_sequence < event.aggregate_sequence
                AND earlier.status <> 'PUBLISHED'
          )
        ORDER BY event.available_at ASC, event.organization_id ASC, event.aggregate_type ASC,
                 event.aggregate_id ASC, event.aggregate_sequence ASC
        LIMIT p_batch_size
        FOR UPDATE SKIP LOCKED
    )
    UPDATE __SCHEMA__.outbox_events AS event
    SET status = 'CLAIMED',
        locked_at = v_now,
        attempt_count = event.attempt_count + 1,
        claim_generation = event.claim_generation + 1,
        claim_token = gen_random_uuid()::text
    FROM candidates
    WHERE event.event_id = candidates.event_id
    RETURNING event.*;
END;
$$;

-- Marks the event published only when the caller still owns the lease.
CREATE FUNCTION __SCHEMA__.outbox_mark_published(
    p_event_id TEXT,
    p_claim_token TEXT
)
RETURNS BOOLEAN
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, __SCHEMA__
AS $$
BEGIN
    UPDATE __SCHEMA__.outbox_events
    SET status = 'PUBLISHED',
        published_at = NOW(),
        locked_at = NULL,
        claim_token = NULL,
        last_error_class = NULL
    WHERE event_id = p_event_id
      AND status = 'CLAIMED'
      AND claim_token = p_claim_token;

    RETURN FOUND;
END;
$$;

-- Releases the event for a retry only when the caller still owns the lease.
CREATE FUNCTION __SCHEMA__.outbox_release_for_retry(
    p_event_id TEXT,
    p_claim_token TEXT,
    p_error_class TEXT,
    p_backoff_interval INTERVAL
)
RETURNS BOOLEAN
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, __SCHEMA__
AS $$
BEGIN
    IF p_error_class IS NULL
       OR p_error_class !~ '^[a-z][a-z0-9_.-]{0,63}$' THEN
        RAISE EXCEPTION 'invalid outbox error class';
    END IF;
    IF p_backoff_interval <= INTERVAL '0'
       OR p_backoff_interval > INTERVAL '1 hour' THEN
        RAISE EXCEPTION 'invalid outbox backoff interval';
    END IF;

    UPDATE __SCHEMA__.outbox_events
    SET status = 'PENDING',
        available_at = NOW() + p_backoff_interval,
        locked_at = NULL,
        claim_token = NULL,
        last_error_class = p_error_class
    WHERE event_id = p_event_id
      AND status = 'CLAIMED'
      AND claim_token = p_claim_token;

    RETURN FOUND;
END;
$$;

-- Quarantines the event only when the caller still owns the lease.
CREATE FUNCTION __SCHEMA__.outbox_quarantine(
    p_event_id TEXT,
    p_claim_token TEXT,
    p_error_class TEXT
)
RETURNS BOOLEAN
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, __SCHEMA__
AS $$
BEGIN
    IF p_error_class IS NULL
       OR p_error_class !~ '^[a-z][a-z0-9_.-]{0,63}$' THEN
        RAISE EXCEPTION 'invalid outbox error class';
    END IF;

    UPDATE __SCHEMA__.outbox_events
    SET status = 'QUARANTINED',
        locked_at = NULL,
        claim_token = NULL,
        last_error_class = p_error_class
    WHERE event_id = p_event_id
      AND status = 'CLAIMED'
      AND claim_token = p_claim_token;

    RETURN FOUND;
END;
$$;

-- Recovers expired leases. Incrementing claim_generation and clearing the
-- token fences a stalled worker that still holds the old token.
CREATE FUNCTION __SCHEMA__.outbox_recover_stale_claims(
    p_lease_timeout INTERVAL
)
RETURNS INTEGER
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, __SCHEMA__
AS $$
DECLARE
    v_count INTEGER;
BEGIN
    IF p_lease_timeout <= INTERVAL '0'
       OR p_lease_timeout > INTERVAL '1 hour' THEN
        RAISE EXCEPTION 'invalid outbox lease timeout';
    END IF;

    UPDATE __SCHEMA__.outbox_events
    SET status = 'PENDING',
        locked_at = NULL,
        claim_token = NULL,
        claim_generation = claim_generation + 1,
        last_error_class = 'lease_expired'
    WHERE status = 'CLAIMED'
      AND locked_at < NOW() - p_lease_timeout;

    GET DIAGNOSTICS v_count = ROW_COUNT;
    RETURN v_count;
END;
$$;

ALTER FUNCTION __SCHEMA__.outbox_claim_events(INTEGER, INTERVAL, INTEGER)
    OWNER TO app_outbox_owner;
ALTER FUNCTION __SCHEMA__.outbox_mark_published(TEXT, TEXT)
    OWNER TO app_outbox_owner;
ALTER FUNCTION __SCHEMA__.outbox_release_for_retry(TEXT, TEXT, TEXT, INTERVAL)
    OWNER TO app_outbox_owner;
ALTER FUNCTION __SCHEMA__.outbox_quarantine(TEXT, TEXT, TEXT)
    OWNER TO app_outbox_owner;
ALTER FUNCTION __SCHEMA__.outbox_recover_stale_claims(INTERVAL)
    OWNER TO app_outbox_owner;

-- Remove PostgreSQL's default EXECUTE grant to PUBLIC, then grant only the
-- worker role. `app_runtime` cannot invoke worker transitions.
REVOKE ALL ON ALL FUNCTIONS IN SCHEMA __SCHEMA__ FROM PUBLIC;

GRANT EXECUTE ON FUNCTION __SCHEMA__.outbox_claim_events(INTEGER, INTERVAL, INTEGER)
    TO app_worker;
GRANT EXECUTE ON FUNCTION __SCHEMA__.outbox_mark_published(TEXT, TEXT)
    TO app_worker;
GRANT EXECUTE ON FUNCTION __SCHEMA__.outbox_release_for_retry(TEXT, TEXT, TEXT, INTERVAL)
    TO app_worker;
GRANT EXECUTE ON FUNCTION __SCHEMA__.outbox_quarantine(TEXT, TEXT, TEXT)
    TO app_worker;
GRANT EXECUTE ON FUNCTION __SCHEMA__.outbox_recover_stale_claims(INTERVAL)
    TO app_worker;

-- ============================================================================
-- OBSERVABILITY VIEW
-- ============================================================================
--
-- The view is intentionally a narrowly scoped owner-rights view: it exposes
-- only status counts and age, never event payloads, actor data, or claim
-- tokens. It is owned by the dedicated outbox role, not the migration login.

CREATE VIEW __SCHEMA__.outbox_metrics AS
SELECT
    status,
    COUNT(*) AS event_count,
    MIN(available_at) AS oldest_pending,
    EXTRACT(EPOCH FROM (NOW() - MIN(available_at))) AS oldest_pending_age_seconds
FROM __SCHEMA__.outbox_events
WHERE status IN ('PENDING', 'CLAIMED')
GROUP BY status;

ALTER VIEW __SCHEMA__.outbox_metrics OWNER TO app_outbox_owner;
REVOKE ALL ON __SCHEMA__.outbox_metrics FROM PUBLIC;
GRANT SELECT ON __SCHEMA__.outbox_metrics TO app_worker;

-- Ownership transfers above require CREATE on the schema. Revoke it last so
-- the dedicated owner cannot create new objects after the fixture is applied.
REVOKE CREATE ON SCHEMA __SCHEMA__ FROM app_outbox_owner;
