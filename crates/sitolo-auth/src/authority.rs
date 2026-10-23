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
    pub session: Session,
    pub user_security_version: SecurityVersion,
}

/// The minimum durable authority needed to turn a verified external identity
/// plus a session selector into a Sitolo principal. Implementations must
/// perform session ownership, suspension, security-version, device, and
/// expiration checks against authoritative state.
#[async_trait]
pub trait RequestIdentityAuthority: Send + Sync {
    async fn resolve_external_identity(
        &self,
        issuer: &str,
        subject: &str,
    ) -> Result<Option<UserId>, AuthError>;

    async fn accept_session_for_user(
        &self,
        id: &SessionId,
        expected_user_id: &UserId,
        now: std::time::SystemTime,
    ) -> Result<SessionSnapshot, AuthError>;

    async fn device_snapshot(
        &self,
        id: &DeviceId,
        expected_user_id: &UserId,
    ) -> Result<Option<Device>, AuthError>;
}
