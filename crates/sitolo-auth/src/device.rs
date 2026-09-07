//! Device identity as an independently revocable security subject.
//!
//! Phase 3 specification, §24-§29, Appendix A.2. A device identifier is never
//! proof of authentication (§0.10); registration is authenticated enrollment
//! (§25); revocation is authoritative server state (§27); replacement creates
//! a new device rather than mutating the old one (§28).

use std::time::SystemTime;

use crate::error::AuthError;
use crate::id::{ClientPlatform, DeviceId, InstallationId, SecurityVersion, UserId};

/// Device lifecycle states (§24.1, A.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeviceState {
    RegistrationStarted,
    VerificationRequired,
    Registered,
    Active,
    Suspended,
    Revoked,
    Replaced,
    Retired,
}

impl DeviceState {
    /// Terminal states can never return to `Active` (A.2; §56 property).
    #[must_use]
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            DeviceState::Revoked | DeviceState::Replaced | DeviceState::Retired
        )
    }

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            DeviceState::RegistrationStarted => "registration_started",
            DeviceState::VerificationRequired => "verification_required",
            DeviceState::Registered => "registered",
            DeviceState::Active => "active",
            DeviceState::Suspended => "suspended",
            DeviceState::Revoked => "revoked",
            DeviceState::Replaced => "replaced",
            DeviceState::Retired => "retired",
        }
    }
}

/// A device record (§24). Hardware identifiers are selected conservatively;
/// none of these fields is ever accepted as proof of possession alone.
#[derive(Debug, Clone)]
pub struct Device {
    pub id: DeviceId,
    pub installation_id: Option<InstallationId>,
    /// Set only from trusted authority after enrollment rules succeed (§25).
    pub organization_ref: Option<String>,
    pub registered_by: UserId,
    pub platform: ClientPlatform,
    pub app_version: Option<String>,
    pub os_version: Option<String>,
    /// Reference to public verification material only; private device keys
    /// never leave platform secure facilities (§26).
    pub device_key_reference: Option<String>,
    pub state: DeviceState,
    pub security_version: SecurityVersion,
    pub registered_at: SystemTime,
    pub last_seen_at: SystemTime,
    pub revoked_at: Option<SystemTime>,
    pub replacement_of: Option<DeviceId>,
}

impl Device {
    /// Server-created registration record. The device identifier is issued
    /// *after* the authoritative row exists (§25); client-chosen identifiers
    /// are structurally impossible at this boundary.
    #[must_use]
    pub fn begin_registration(
        id: DeviceId,
        registered_by: UserId,
        platform: ClientPlatform,
        security_version: SecurityVersion,
        now: SystemTime,
    ) -> Device {
        Device {
            id,
            installation_id: None,
            organization_ref: None,
            registered_by,
            platform,
            app_version: None,
            os_version: None,
            device_key_reference: None,
            state: DeviceState::RegistrationStarted,
            security_version,
            registered_at: now,
            last_seen_at: now,
            revoked_at: None,
            replacement_of: None,
        }
    }

    fn transition(&mut self, from: &[DeviceState], to: DeviceState) -> Result<(), AuthError> {
        if from.contains(&self.state) {
            self.state = to;
            Ok(())
        } else {
            Err(AuthError::InvalidTransition)
        }
    }

    pub fn require_verification(&mut self) -> Result<(), AuthError> {
        self.transition(
            &[DeviceState::RegistrationStarted],
            DeviceState::VerificationRequired,
        )
    }

    pub fn complete_verification(&mut self) -> Result<(), AuthError> {
        self.transition(
            &[DeviceState::VerificationRequired],
            DeviceState::Registered,
        )
    }

    pub fn activate(&mut self) -> Result<(), AuthError> {
        self.transition(&[DeviceState::Registered], DeviceState::Active)
    }

    pub fn suspend(&mut self) -> Result<(), AuthError> {
        self.transition(&[DeviceState::Active], DeviceState::Suspended)
    }

    /// Suspended devices may be reinstated; revoked ones never are (§27, A.2).
    pub fn reinstate(&mut self) -> Result<(), AuthError> {
        self.transition(&[DeviceState::Suspended], DeviceState::Active)
    }

    /// Atomic revocation with security-version increment (§27). Returns the
    /// post-increment version for dependent-session invalidation.
    pub fn revoke(&mut self, now: SystemTime) -> Result<SecurityVersion, AuthError> {
        if self.state.is_terminal() {
            return Err(AuthError::InvalidTransition);
        }
        self.state = DeviceState::Revoked;
        self.revoked_at = Some(now);
        self.security_version = self.security_version.next();
        Ok(self.security_version)
    }

    /// Retire on replacement; the old record stays as evidence (§28).
    pub fn retire_for_replacement(&mut self, now: SystemTime) -> Result<(), AuthError> {
        if self.state.is_terminal() {
            return Err(AuthError::InvalidTransition);
        }
        self.state = DeviceState::Retired;
        self.revoked_at = Some(now);
        self.security_version = self.security_version.next();
        Ok(())
    }

    /// Only `ACTIVE` devices participate in device-required operations (§24.1).
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.state == DeviceState::Active
    }
}

/// The durable outcome of a revocation (§27, §52.3).
#[derive(Debug, Clone)]
pub struct DeviceRevocationEffect {
    pub device: Device,
    /// Sessions revoked because they were bound to this device.
    pub revoked_sessions: Vec<crate::id::SessionId>,
    /// Offline capability windows that became invalid (§27, §30).
    pub offline_capability_invalidated: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn device() -> Device {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(500);
        Device::begin_registration(
            DeviceId::new("dev-01").unwrap(),
            UserId::new("user-01").unwrap(),
            ClientPlatform::Android,
            SecurityVersion(1),
            now,
        )
    }

    #[test]
    fn lifecycle_follows_appendix_a2() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(500);
        let mut d = device();
        assert_eq!(d.state, DeviceState::RegistrationStarted);
        d.require_verification().unwrap();
        d.complete_verification().unwrap();
        d.activate().unwrap();
        d.suspend().unwrap();
        d.reinstate().unwrap();
        let bumped = d.revoke(now + Duration::from_secs(10)).unwrap();
        assert_eq!(bumped, SecurityVersion(2));
        assert_eq!(d.state, DeviceState::Revoked);
        assert!(d.revoked_at.is_some());
    }

    #[test]
    fn revoked_device_never_becomes_active() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(500);
        let mut d = device();
        d.require_verification().unwrap();
        d.complete_verification().unwrap();
        d.activate().unwrap();
        d.revoke(now).unwrap();
        assert!(d.activate().is_err());
        assert!(d.reinstate().is_err());
        assert!(matches!(d.revoke(now), Err(AuthError::InvalidTransition)));
    }

    #[test]
    fn registration_cannot_skip_verification() {
        let mut d = device();
        assert!(matches!(d.activate(), Err(AuthError::InvalidTransition)));
        assert!(matches!(
            d.complete_verification(),
            Err(AuthError::InvalidTransition)
        ));
    }

    #[test]
    fn replacement_retires_old_without_reviving() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(500);
        let mut old = device();
        old.require_verification().unwrap();
        old.complete_verification().unwrap();
        old.activate().unwrap();
        old.retire_for_replacement(now).unwrap();
        assert_eq!(old.state, DeviceState::Retired);
        let mut fresh = device();
        fresh.replacement_of = Some(old.id.clone());
        assert_eq!(fresh.state, DeviceState::RegistrationStarted);
    }
}
