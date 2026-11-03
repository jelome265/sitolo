-- Persist explicit current-session logout with atomic, immutable audit evidence.
ALTER TABLE __SCHEMA__.identity_sessions
    ADD COLUMN revoked_at TIMESTAMPTZ,
    ADD COLUMN revoke_reason TEXT,
    ADD CONSTRAINT identity_sessions_revoke_metadata_pair
        CHECK ((revoked_at IS NULL) = (revoke_reason IS NULL)),
    ADD CONSTRAINT identity_sessions_revoke_reason_check
        CHECK (revoke_reason IS NULL OR revoke_reason = 'logout');

ALTER TABLE __SCHEMA__.identity_auth_events
    DROP CONSTRAINT identity_auth_events_event_name_check,
    ADD CONSTRAINT identity_auth_events_event_name_check
        CHECK (event_name IN ('auth.session.expired', 'auth.session.revoked')),
    DROP CONSTRAINT identity_auth_events_reason_class_check,
    ADD CONSTRAINT identity_auth_events_reason_class_check
        CHECK (reason_class IN ('idle_expired', 'absolute_expired', 'logout'));

CREATE FUNCTION __SCHEMA__.revoke_sitolo_session(
    p_session_id TEXT,
    p_expected_user_id TEXT,
    p_now TIMESTAMPTZ
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
DECLARE
    v_session __SCHEMA__.identity_sessions%ROWTYPE;
BEGIN
    UPDATE __SCHEMA__.identity_sessions AS s
       SET state = 'revoked',
           revoked_at = p_now,
           revoke_reason = 'logout'
     WHERE s.id = p_session_id
       AND s.user_id = p_expected_user_id
       AND s.state = 'active'
    RETURNING s.* INTO v_session;

    IF NOT FOUND THEN
        RETURN;
    END IF;

    INSERT INTO __SCHEMA__.identity_auth_events (
        event_id, event_name, occurred_at, subject_ref, session_ref,
        result, reason_class, security_version
    ) VALUES (
        v_session.id || ':revoked',
        'auth.session.revoked',
        p_now,
        v_session.user_id,
        v_session.id,
        'SUCCESS',
        'logout',
        v_session.security_version
    );

    RETURN;
END
$$;

ALTER FUNCTION __SCHEMA__.revoke_sitolo_session(TEXT, TEXT, TIMESTAMPTZ)
    OWNER TO app_identity_owner;
REVOKE ALL ON FUNCTION __SCHEMA__.revoke_sitolo_session(TEXT, TEXT, TIMESTAMPTZ)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION __SCHEMA__.revoke_sitolo_session(TEXT, TEXT, TIMESTAMPTZ)
    TO app_runtime;
