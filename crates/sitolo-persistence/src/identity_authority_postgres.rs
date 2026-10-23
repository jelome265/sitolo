//! PostgreSQL-backed request identity mapping and Sitolo session authority.
//!
//! External identities are lookup-only at request time. Session acceptance is
//! delegated to a narrow database function so ownership, account state,
//! security version, device state, expiry and idle sliding are evaluated
//! against the same authoritative PostgreSQL operation.

use std::time::SystemTime;

use chrono::{DateTime, Utc};
use sitolo_auth::{
    Assurance, AuthenticationMethod, ClientPlatform, Device, DeviceId, DeviceState, InstallationId,
    RequestIdentityAuthority, SecurityVersion, Session, SessionClass, SessionId, SessionSnapshot,
    SessionState, UserId,
};
use sqlx_core::query::query;
use sqlx_core::query_scalar::query_scalar;
use sqlx_core::row::Row;
use sqlx_postgres::PgPool;

use crate::postgres_time::from_utc;

/// Least-privilege request identity repository.
#[derive(Clone)]
pub struct PgRequestIdentityAuthority {
    runtime_pool: PgPool,
}

impl PgRequestIdentityAuthority {
    /// The pool must authenticate directly as `app_runtime`, have the target
    /// schema pinned as its `search_path`, and prove both `current_user` and
    /// `session_user` before production composition constructs this repository.
    pub fn new(
        runtime_pool: PgPool,
        schema_name: impl Into<String>,
    ) -> Result<Self, sitolo_auth::AuthError> {
        let schema_name = schema_name.into();
        if !valid_schema_name(&schema_name) {
            return Err(sitolo_auth::AuthError::ConfigurationInvalid);
        }
        Ok(Self { runtime_pool })
    }
}

#[async_trait::async_trait]
impl RequestIdentityAuthority for PgRequestIdentityAuthority {
    async fn resolve_external_identity(
        &self,
        issuer: &str,
        subject: &str,
    ) -> Result<Option<UserId>, sitolo_auth::AuthError> {
        let resolved: Option<String> = query_scalar("SELECT resolve_external_identity($1, $2)")
            .bind(issuer)
            .bind(subject)
            .fetch_one(&self.runtime_pool)
            .await
            .map_err(|_| sitolo_auth::AuthError::IdentityAuthorityUnavailable)?;
        resolved
            .map(|id| {
                UserId::new(id).map_err(|_| sitolo_auth::AuthError::IdentityAuthorityUnavailable)
            })
            .transpose()
    }

    async fn accept_session_for_user(
        &self,
        id: &SessionId,
        expected_user_id: &UserId,
        now: SystemTime,
    ) -> Result<SessionSnapshot, sitolo_auth::AuthError> {
        let now = crate::postgres_time::to_utc(now)
            .ok_or(sitolo_auth::AuthError::AuthenticationFailed)?;
        let row = query("SELECT * FROM accept_sitolo_session($1, $2, $3)")
            .bind(id.as_str())
            .bind(expected_user_id.as_str())
            .bind(now)
            .fetch_optional(&self.runtime_pool)
            .await
            .map_err(|_| sitolo_auth::AuthError::IdentityAuthorityUnavailable)?
            .ok_or(sitolo_auth::AuthError::AuthenticationFailed)?;

        let session_id = SessionId::new(
            row.try_get::<String, _>("session_id")
                .map_err(|_| sitolo_auth::AuthError::IdentityAuthorityUnavailable)?,
        )
        .map_err(|_| sitolo_auth::AuthError::IdentityAuthorityUnavailable)?;
        let user_id = UserId::new(
            row.try_get::<String, _>("user_id")
                .map_err(|_| sitolo_auth::AuthError::IdentityAuthorityUnavailable)?,
        )
        .map_err(|_| sitolo_auth::AuthError::IdentityAuthorityUnavailable)?;
        let device_id = row
            .try_get::<Option<String>, _>("device_id")
            .map_err(|_| sitolo_auth::AuthError::IdentityAuthorityUnavailable)?
            .map(DeviceId::new)
            .transpose()
            .map_err(|_| sitolo_auth::AuthError::IdentityAuthorityUnavailable)?;
        let session = Session {
            id: session_id,
            user_id,
            device_id,
            class: parse_session_class(&row)?,
            state: parse_session_state(&row)?,
            created_at: get_time(&row, "created_at")?,
            authenticated_at: get_time(&row, "authenticated_at")?,
            last_seen_at: get_time(&row, "last_seen_at")?,
            idle_expires_at: get_time(&row, "idle_expires_at")?,
            absolute_expires_at: get_time(&row, "absolute_expires_at")?,
            assurance: parse_assurance(&row)?,
            authentication_method: parse_authentication_method(&row)?,
            security_version: get_version(&row, "security_version")?,
        };
        Ok(SessionSnapshot {
            session,
            user_security_version: get_version(&row, "user_security_version")?,
        })
    }

    async fn device_snapshot(
        &self,
        id: &DeviceId,
        expected_user_id: &UserId,
    ) -> Result<Option<Device>, sitolo_auth::AuthError> {
        let row = query("SELECT * FROM load_identity_device($1, $2)")
            .bind(id.as_str())
            .bind(expected_user_id.as_str())
            .fetch_optional(&self.runtime_pool)
            .await
            .map_err(|_| sitolo_auth::AuthError::IdentityAuthorityUnavailable)?;
        row.map(|row| {
            let user_id = UserId::new(
                row.try_get::<String, _>("registered_by")
                    .map_err(|_| sitolo_auth::AuthError::IdentityAuthorityUnavailable)?,
            )
            .map_err(|_| sitolo_auth::AuthError::IdentityAuthorityUnavailable)?;
            Ok(Device {
                id: DeviceId::new(
                    row.try_get::<String, _>("id")
                        .map_err(|_| sitolo_auth::AuthError::IdentityAuthorityUnavailable)?,
                )
                .map_err(|_| sitolo_auth::AuthError::IdentityAuthorityUnavailable)?,
                installation_id: row
                    .try_get::<Option<String>, _>("installation_id")
                    .map_err(|_| sitolo_auth::AuthError::IdentityAuthorityUnavailable)?
                    .map(InstallationId::new)
                    .transpose()
                    .map_err(|_| sitolo_auth::AuthError::IdentityAuthorityUnavailable)?,
                organization_ref: row
                    .try_get("organization_ref")
                    .map_err(|_| sitolo_auth::AuthError::IdentityAuthorityUnavailable)?,
                registered_by: user_id,
                platform: parse_platform(&row)?,
                app_version: row
                    .try_get("app_version")
                    .map_err(|_| sitolo_auth::AuthError::IdentityAuthorityUnavailable)?,
                os_version: row
                    .try_get("os_version")
                    .map_err(|_| sitolo_auth::AuthError::IdentityAuthorityUnavailable)?,
                device_key_reference: row
                    .try_get("device_key_reference")
                    .map_err(|_| sitolo_auth::AuthError::IdentityAuthorityUnavailable)?,
                state: parse_device_state(&row)?,
                security_version: get_version(&row, "security_version")?,
                registered_at: get_time(&row, "registered_at")?,
                last_seen_at: get_time(&row, "last_seen_at")?,
                revoked_at: row
                    .try_get::<Option<DateTime<Utc>>, _>("revoked_at")
                    .map_err(|_| sitolo_auth::AuthError::IdentityAuthorityUnavailable)?
                    .map(|value| {
                        from_utc(value).ok_or(sitolo_auth::AuthError::IdentityAuthorityUnavailable)
                    })
                    .transpose()?,
                replacement_of: row
                    .try_get::<Option<String>, _>("replacement_of")
                    .map_err(|_| sitolo_auth::AuthError::IdentityAuthorityUnavailable)?
                    .map(DeviceId::new)
                    .transpose()
                    .map_err(|_| sitolo_auth::AuthError::IdentityAuthorityUnavailable)?,
            })
        })
        .transpose()
    }
}

fn valid_schema_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 63
        && value
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_lowercase() || byte == b'_')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

fn get_time(row: &sqlx_postgres::PgRow, name: &str) -> Result<SystemTime, sitolo_auth::AuthError> {
    let value: DateTime<Utc> = row
        .try_get(name)
        .map_err(|_| sitolo_auth::AuthError::IdentityAuthorityUnavailable)?;
    from_utc(value).ok_or(sitolo_auth::AuthError::IdentityAuthorityUnavailable)
}

fn get_version(
    row: &sqlx_postgres::PgRow,
    name: &str,
) -> Result<SecurityVersion, sitolo_auth::AuthError> {
    let value: i64 = row
        .try_get(name)
        .map_err(|_| sitolo_auth::AuthError::IdentityAuthorityUnavailable)?;
    u64::try_from(value)
        .map(SecurityVersion)
        .map_err(|_| sitolo_auth::AuthError::IdentityAuthorityUnavailable)
}

fn text(row: &sqlx_postgres::PgRow, name: &str) -> Result<String, sitolo_auth::AuthError> {
    row.try_get(name)
        .map_err(|_| sitolo_auth::AuthError::IdentityAuthorityUnavailable)
}

fn parse_session_class(row: &sqlx_postgres::PgRow) -> Result<SessionClass, sitolo_auth::AuthError> {
    match text(row, "session_class")?.as_str() {
        "merchant" => Ok(SessionClass::Merchant),
        "admin" => Ok(SessionClass::Administrator),
        "break_glass" => Ok(SessionClass::BreakGlass),
        _ => Err(sitolo_auth::AuthError::IdentityAuthorityUnavailable),
    }
}

fn parse_session_state(row: &sqlx_postgres::PgRow) -> Result<SessionState, sitolo_auth::AuthError> {
    match text(row, "session_state")?.as_str() {
        "created" => Ok(SessionState::Created),
        "active" => Ok(SessionState::Active),
        "idle_expired" => Ok(SessionState::IdleExpired),
        "absolute_expired" => Ok(SessionState::AbsoluteExpired),
        "revoked" => Ok(SessionState::Revoked),
        "security_revoked" => Ok(SessionState::SecurityRevoked),
        _ => Err(sitolo_auth::AuthError::IdentityAuthorityUnavailable),
    }
}

fn parse_assurance(row: &sqlx_postgres::PgRow) -> Result<Assurance, sitolo_auth::AuthError> {
    match text(row, "assurance")?.as_str() {
        "a0" => Ok(Assurance::A0),
        "a1" => Ok(Assurance::A1),
        "a2" => Ok(Assurance::A2),
        "a3" => Ok(Assurance::A3),
        "a4" => Ok(Assurance::A4),
        _ => Err(sitolo_auth::AuthError::IdentityAuthorityUnavailable),
    }
}

fn parse_authentication_method(
    row: &sqlx_postgres::PgRow,
) -> Result<AuthenticationMethod, sitolo_auth::AuthError> {
    match text(row, "authentication_method")?.as_str() {
        "password" => Ok(AuthenticationMethod::Password),
        "oidc" => Ok(AuthenticationMethod::OidcAssertion),
        "passkey" => Ok(AuthenticationMethod::Passkey),
        "provider_managed" => Ok(AuthenticationMethod::ProviderManaged),
        "recovery_code" => Ok(AuthenticationMethod::RecoveryCode),
        "service" => Ok(AuthenticationMethod::ServiceCredential),
        _ => Err(sitolo_auth::AuthError::IdentityAuthorityUnavailable),
    }
}

fn parse_device_state(row: &sqlx_postgres::PgRow) -> Result<DeviceState, sitolo_auth::AuthError> {
    match text(row, "state")?.as_str() {
        "registration_started" => Ok(DeviceState::RegistrationStarted),
        "verification_required" => Ok(DeviceState::VerificationRequired),
        "registered" => Ok(DeviceState::Registered),
        "active" => Ok(DeviceState::Active),
        "suspended" => Ok(DeviceState::Suspended),
        "revoked" => Ok(DeviceState::Revoked),
        "replaced" => Ok(DeviceState::Replaced),
        "retired" => Ok(DeviceState::Retired),
        _ => Err(sitolo_auth::AuthError::IdentityAuthorityUnavailable),
    }
}

fn parse_platform(row: &sqlx_postgres::PgRow) -> Result<ClientPlatform, sitolo_auth::AuthError> {
    match text(row, "platform")?.as_str() {
        "android" => Ok(ClientPlatform::Android),
        "ios" => Ok(ClientPlatform::Ios),
        "desktop" => Ok(ClientPlatform::Desktop),
        "web" => Ok(ClientPlatform::Web),
        "service" => Ok(ClientPlatform::Service),
        _ => Err(sitolo_auth::AuthError::IdentityAuthorityUnavailable),
    }
}

#[cfg(test)]
mod tests {
    use super::valid_schema_name;

    #[test]
    fn schema_name_is_a_bounded_trusted_identifier() {
        assert!(valid_schema_name("public"));
        assert!(valid_schema_name("test_part8_a1b2c3"));
        assert!(!valid_schema_name(""));
        assert!(!valid_schema_name("Public"));
        assert!(!valid_schema_name("test.schema"));
        assert!(!valid_schema_name("x\"; DROP SCHEMA public;--"));
    }
}
