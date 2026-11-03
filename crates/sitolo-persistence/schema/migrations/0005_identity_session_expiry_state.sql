-- Persist terminal session-expiry states when an expired session is presented.
-- Kept as an additive migration so released migration 0001 remains immutable.
CREATE TABLE __SCHEMA__.identity_auth_events (
    event_id TEXT PRIMARY KEY,
    event_name TEXT NOT NULL CHECK (event_name = 'auth.session.expired'),
    occurred_at TIMESTAMPTZ NOT NULL,
    subject_ref TEXT NOT NULL,
    session_ref TEXT NOT NULL,
    result TEXT NOT NULL CHECK (result = 'SUCCESS'),
    reason_class TEXT NOT NULL CHECK (reason_class IN ('idle_expired', 'absolute_expired')),
    security_version BIGINT NOT NULL CHECK (security_version > 0),
    CONSTRAINT uq_identity_auth_event_session UNIQUE (session_ref, event_name)
);

ALTER TABLE __SCHEMA__.identity_auth_events OWNER TO app_identity_owner;
REVOKE ALL ON __SCHEMA__.identity_auth_events FROM PUBLIC, app_runtime;

CREATE OR REPLACE FUNCTION __SCHEMA__.accept_sitolo_session(
    p_session_id TEXT,
    p_expected_user_id TEXT,
    p_now TIMESTAMPTZ
)
RETURNS TABLE (
    session_id TEXT,
    user_id TEXT,
    device_id TEXT,
    session_class TEXT,
    session_state TEXT,
    created_at TIMESTAMPTZ,
    authenticated_at TIMESTAMPTZ,
    last_seen_at TIMESTAMPTZ,
    idle_expires_at TIMESTAMPTZ,
    absolute_expires_at TIMESTAMPTZ,
    assurance TEXT,
    authentication_method TEXT,
    security_version BIGINT,
    user_security_version BIGINT,
    device_state TEXT,
    device_security_version BIGINT
)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
DECLARE
    v_session __SCHEMA__.identity_sessions%ROWTYPE;
    v_user_version BIGINT;
    v_device_state TEXT;
    v_device_version BIGINT;
    v_expiry_reason TEXT;
BEGIN
    -- Expiry is a terminal state transition, but only for the matching,
    -- current user/session pair. A stale session cannot revoke a newer or
    -- differently owned session by supplying another user's identifier.
    UPDATE __SCHEMA__.identity_sessions AS s
       SET state = CASE
           WHEN s.absolute_expires_at <= p_now THEN 'absolute_expired'
           ELSE 'idle_expired'
       END
      FROM __SCHEMA__.identity_users AS u
     WHERE s.id = p_session_id
       AND s.user_id = p_expected_user_id
       AND u.id = s.user_id
       AND u.suspended = FALSE
       AND s.state = 'active'
       AND s.security_version = u.security_version
       AND (s.idle_expires_at <= p_now OR s.absolute_expires_at <= p_now)
    RETURNING s.* INTO v_session;

    IF FOUND THEN
        v_expiry_reason := v_session.state;
        INSERT INTO __SCHEMA__.identity_auth_events (
            event_id, event_name, occurred_at, subject_ref, session_ref,
            result, reason_class, security_version
        ) VALUES (
            v_session.id,
            'auth.session.expired', p_now, v_session.user_id, v_session.id,
            'SUCCESS', v_expiry_reason, v_session.security_version
        );
    END IF;

    SELECT s.*
      INTO v_session
      FROM __SCHEMA__.identity_sessions AS s
      JOIN __SCHEMA__.identity_users AS u ON u.id = s.user_id
     WHERE s.id = p_session_id
       AND s.user_id = p_expected_user_id
       AND u.suspended = FALSE
       AND s.state = 'active'
       AND s.security_version = u.security_version
       AND s.idle_expires_at > p_now
       AND s.absolute_expires_at > p_now
     FOR UPDATE OF s, u;

    IF NOT FOUND THEN
        RETURN;
    END IF;

    SELECT u.security_version
      INTO v_user_version
      FROM __SCHEMA__.identity_users AS u
     WHERE u.id = v_session.user_id
       AND u.suspended = FALSE
       AND u.security_version = v_session.security_version
     FOR UPDATE;
    IF NOT FOUND THEN
        RETURN;
    END IF;

    IF v_session.device_id IS NOT NULL THEN
        SELECT d.state, d.security_version
          INTO v_device_state, v_device_version
          FROM __SCHEMA__.identity_devices AS d
         WHERE d.id = v_session.device_id
           AND d.user_id = v_session.user_id
         FOR UPDATE;
        IF NOT FOUND OR v_device_state <> 'active' THEN
            RETURN;
        END IF;
    END IF;

    UPDATE __SCHEMA__.identity_sessions AS s
       SET last_seen_at = p_now,
           idle_expires_at = LEAST(
               p_now + make_interval(secs => v_session.idle_timeout_seconds::DOUBLE PRECISION),
               v_session.absolute_expires_at
           )
     WHERE s.id = p_session_id
     RETURNING s.* INTO v_session;

    RETURN QUERY SELECT
        v_session.id, v_session.user_id, v_session.device_id,
        v_session.session_class, v_session.state, v_session.created_at,
        v_session.authenticated_at, v_session.last_seen_at,
        v_session.idle_expires_at, v_session.absolute_expires_at,
        v_session.assurance, v_session.authentication_method,
        v_session.security_version, v_user_version,
        v_device_state, v_device_version;
END
$$;

ALTER FUNCTION __SCHEMA__.accept_sitolo_session(TEXT, TEXT, TIMESTAMPTZ) OWNER TO app_identity_owner;
REVOKE ALL ON FUNCTION __SCHEMA__.accept_sitolo_session(TEXT, TEXT, TIMESTAMPTZ) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION __SCHEMA__.accept_sitolo_session(TEXT, TEXT, TIMESTAMPTZ) TO app_runtime;
