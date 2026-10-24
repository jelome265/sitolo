//! Protocol-neutral port for resolving a verified identity to durable Sitolo session authority.
//!
//! The API layer consumes this contract; PostgreSQL implementations live in
//! sitolo-persistence. The port belongs to the auth boundary and must not
//! expose database-specific types.

use async_trait::async_trait;

use crate::device::Device;
use crate::error::AuthError;
use crate::id::{DeviceId, SecurityVersion, SessionId, UserId};
use crate::session::Session;

/// Authoritative snapshot needed to construct a security context.
#[derive(Debug, Clone)]
pub struct SessionSnapshot {
    /// The durable session row as stored by the authority.
    pub session: Session,
    /// The owning user's current security version, used to detect stale
    /// authority (§31).
    pub user_security_version: SecurityVersion,
}

/// The minimum durable authority needed to turn a verified external identity
/// plus a session selector into a Sitolo principal. Implementations must
/// perform session ownership, suspension, security-version, device, and
/// expiration checks against authoritative state.
#[async_trait]
pub trait RequestIdentityAuthority: Send + Sync {
    /// Resolves an external `(issuer, subject)` pair to the owning user.
    ///
    /// # Errors
    ///
    /// Returns [`AuthError::IdentityAuthorityUnavailable`] when authoritative
    /// state cannot be read. An unknown identity is not an error and yields
    /// `Ok(None)`.
    async fn resolve_external_identity(
        &self,
        issuer: &str,
        subject: &str,
    ) -> Result<Option<UserId>, AuthError>;

    /// Loads a session after verifying that it belongs to `expected_user_id`
    /// and is still acceptable for request handling at `now`.
    ///
    /// # Errors
    ///
    /// Returns [`AuthError::SessionInvalid`] when the session does not exist or
    /// belongs to another user, [`AuthError::SessionExpired`] when it has
    /// expired, [`AuthError::SessionRevoked`] when it is revoked, and
    /// [`AuthError::IdentityAuthorityUnavailable`] when authoritative state
    /// cannot be read.
    async fn accept_session_for_user(
        &self,
        id: &SessionId,
        expected_user_id: &UserId,
        now: std::time::SystemTime,
    ) -> Result<SessionSnapshot, AuthError>;

    /// Loads the device snapshot bound to a session, if the session is
    /// device-bound.
    ///
    /// # Errors
    ///
    /// Returns [`AuthError::IdentityAuthorityUnavailable`] when authoritative
    /// state cannot be read. A session without a registered device yields
    /// `Ok(None)`.
    async fn device_snapshot(
        &self,
        id: &DeviceId,
        expected_user_id: &UserId,
    ) -> Result<Option<Device>, AuthError>;
}
