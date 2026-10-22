-- Phase 3 request identity authority.
-- Apply as the restricted migration identity after app_runtime exists.
-- This schema stores immutable external identity links and Sitolo session
-- state. Token claims never create/link users or grant tenant authority.

DO $$
BEGIN
    PERFORM pg_catalog.pg_advisory_xact_lock(
        pg_catalog.hashtext('sitolo.identity.role-bootstrap'), 8
    );
    CREATE ROLE app_identity_owner NOLOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS;
EXCEPTION WHEN duplicate_object THEN
    NULL;
END
$$;

ALTER ROLE app_identity_owner NOLOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS;

DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM pg_catalog.pg_auth_members AS membership
        JOIN pg_catalog.pg_roles AS owner_role ON owner_role.oid = membership.roleid
        JOIN pg_catalog.pg_roles AS member_role ON member_role.oid = membership.member
        WHERE owner_role.rolname = 'app_identity_owner'
           OR member_role.rolname = 'app_identity_owner'
    ) THEN
        RAISE EXCEPTION 'app_identity_owner must have no role memberships';
    END IF;
END
$$;

GRANT USAGE ON SCHEMA __SCHEMA__ TO app_runtime, app_identity_owner;

CREATE TABLE identity_users (
    id TEXT PRIMARY KEY,
    security_version BIGINT NOT NULL CHECK (security_version > 0),
    suspended BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE TABLE external_identities (
    issuer TEXT NOT NULL CHECK (length(issuer) BETWEEN 1 AND 2048),
    subject TEXT NOT NULL CHECK (length(subject) BETWEEN 1 AND 256),
    user_id TEXT NOT NULL REFERENCES identity_users(id) ON DELETE RESTRICT,
    linked_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (issuer, subject)
);

CREATE INDEX idx_external_identities_user ON external_identities(user_id);

CREATE TABLE identity_devices (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES identity_users(id) ON DELETE RESTRICT,
    installation_id TEXT,
    organization_ref TEXT,
    registered_by TEXT NOT NULL REFERENCES identity_users(id) ON DELETE RESTRICT,
    platform TEXT NOT NULL CHECK (platform IN ('android', 'ios', 'desktop', 'web', 'service')),
    app_version TEXT,
    os_version TEXT,
    device_key_reference TEXT,
    state TEXT NOT NULL CHECK (state IN (
        'registration_started', 'verification_required', 'registered', 'active',
        'suspended', 'revoked', 'replaced', 'retired'
    )),
    security_version BIGINT NOT NULL CHECK (security_version > 0),
    registered_at TIMESTAMPTZ NOT NULL,
    last_seen_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ,
    replacement_of TEXT REFERENCES identity_devices(id) ON DELETE RESTRICT
);

CREATE TABLE identity_sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES identity_users(id) ON DELETE RESTRICT,
    device_id TEXT REFERENCES identity_devices(id) ON DELETE RESTRICT,
    session_class TEXT NOT NULL CHECK (session_class IN ('merchant', 'admin', 'break_glass')),
    state TEXT NOT NULL CHECK (state IN (
        'created', 'active', 'idle_expired', 'absolute_expired', 'revoked', 'security_revoked'
    )),
    created_at TIMESTAMPTZ NOT NULL,
    authenticated_at TIMESTAMPTZ NOT NULL,
    last_seen_at TIMESTAMPTZ NOT NULL,
    idle_expires_at TIMESTAMPTZ NOT NULL,
    absolute_expires_at TIMESTAMPTZ NOT NULL,
    idle_timeout_seconds BIGINT NOT NULL CHECK (idle_timeout_seconds BETWEEN 1 AND 31536000),
    assurance TEXT NOT NULL CHECK (assurance IN ('a0', 'a1', 'a2', 'a3', 'a4')),
    authentication_method TEXT NOT NULL CHECK (authentication_method IN (
        'password', 'oidc', 'passkey', 'provider_managed', 'recovery_code', 'service'
    )),
    security_version BIGINT NOT NULL CHECK (security_version > 0),
    CHECK (idle_expires_at <= absolute_expires_at),
    CHECK (absolute_expires_at > created_at)
);

CREATE INDEX idx_identity_sessions_user_state ON identity_sessions(user_id, state);
CREATE INDEX idx_identity_sessions_device_state ON identity_sessions(device_id, state)
    WHERE device_id IS NOT NULL;

ALTER TABLE identity_users OWNER TO app_identity_owner;
ALTER TABLE external_identities OWNER TO app_identity_owner;
ALTER TABLE identity_devices OWNER TO app_identity_owner;
ALTER TABLE identity_sessions OWNER TO app_identity_owner;

REVOKE ALL ON identity_users, external_identities, identity_devices, identity_sessions FROM PUBLIC;
REVOKE ALL ON identity_users, external_identities, identity_devices, identity_sessions FROM app_runtime;

-- Provider identifiers are exact and pre-linked. This lookup intentionally
-- does not use email, display name, tenant claims, or implicit provisioning.
CREATE FUNCTION __SCHEMA__.resolve_external_identity(p_issuer TEXT, p_subject TEXT)
RETURNS TEXT
LANGUAGE SQL
STABLE
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
    SELECT ei.user_id
    FROM __SCHEMA__.external_identities AS ei
    JOIN __SCHEMA__.identity_users AS iu ON iu.id = ei.user_id
    WHERE ei.issuer = p_issuer
      AND ei.subject = p_subject
      AND iu.suspended = FALSE
$$;

ALTER FUNCTION __SCHEMA__.resolve_external_identity(TEXT, TEXT) OWNER TO app_identity_owner;
REVOKE ALL ON FUNCTION __SCHEMA__.resolve_external_identity(TEXT, TEXT) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION __SCHEMA__.resolve_external_identity(TEXT, TEXT) TO app_runtime;

-- The caller supplies the already-mapped user ID. The function verifies the
-- session belongs to it, is active and unexpired, checks account security
-- version/suspension and bound-device state, then slides idle expiry in the
-- same database operation. Missing/denied rows are indistinguishable.
CREATE FUNCTION __SCHEMA__.accept_sitolo_session(
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
BEGIN
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

-- Device records are returned only when the owning user matches the principal
-- already established by the accepted session.
CREATE FUNCTION __SCHEMA__.load_identity_device(p_device_id TEXT, p_user_id TEXT)
RETURNS TABLE (
    id TEXT,
    installation_id TEXT,
    organization_ref TEXT,
    registered_by TEXT,
    platform TEXT,
    app_version TEXT,
    os_version TEXT,
    device_key_reference TEXT,
    state TEXT,
    security_version BIGINT,
    registered_at TIMESTAMPTZ,
    last_seen_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ,
    replacement_of TEXT
)
LANGUAGE SQL
STABLE
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
    SELECT d.id, d.installation_id, d.organization_ref, d.registered_by,
           d.platform, d.app_version, d.os_version, d.device_key_reference,
           d.state, d.security_version, d.registered_at, d.last_seen_at,
           d.revoked_at, d.replacement_of
    FROM __SCHEMA__.identity_devices AS d
    WHERE d.id = p_device_id AND d.user_id = p_user_id
$$;

ALTER FUNCTION __SCHEMA__.load_identity_device(TEXT, TEXT) OWNER TO app_identity_owner;
REVOKE ALL ON FUNCTION __SCHEMA__.load_identity_device(TEXT, TEXT) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION __SCHEMA__.load_identity_device(TEXT, TEXT) TO app_runtime;
