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
    /// Registration has started but enrollment is not yet verified.
    RegistrationStarted,
    /// The device must complete a verification step before registration.
    VerificationRequired,
    /// Enrollment is verified and the device is registered.
    Registered,
    /// The device may participate in device-required operations (§24.1).
    Active,
    /// The device is temporarily suspended and may be reinstated (§27).
    Suspended,
    /// The device is permanently revoked; this state is terminal (§27).
    Revoked,
    /// The device was replaced by a newer record; this state is terminal
    /// (§28).
    Replaced,
    /// The device was retired without replacement; this state is terminal.
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

    /// Stable bounded label for telemetry (§38).
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
    /// Server-issued device identifier (§24).
    pub id: DeviceId,
    /// Optional installation identifier for this app instance.
    pub installation_id: Option<InstallationId>,
    /// Set only from trusted authority after enrollment rules succeed (§25).
    pub organization_ref: Option<String>,
    /// The user who completed authenticated enrollment (§25).
    pub registered_by: UserId,
    /// Client platform that registered the device.
    pub platform: ClientPlatform,
    /// Reported application version, for support and policy decisions.
    pub app_version: Option<String>,
    /// Reported operating-system version, for support and policy decisions.
    pub os_version: Option<String>,
    /// Reference to public verification material only; private device keys
    /// never leave platform secure facilities (§26).
    pub device_key_reference: Option<String>,
    /// Current lifecycle state (§24.1).
    pub state: DeviceState,
    /// Device security version; incremented by revocation (§31).
    pub security_version: SecurityVersion,
    /// When registration began.
    pub registered_at: SystemTime,
    /// When the device was last accepted for an authenticated request.
    pub last_seen_at: SystemTime,
    /// When the device was revoked or retired, if either happened.
    pub revoked_at: Option<SystemTime>,
    /// Previous device this record replaces, if any (§28).
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

    /// Requires a verification step for a newly started registration
    /// (§24.1).
    ///
    /// # Errors
    ///
    /// Returns [`AuthError::InvalidTransition`] unless the device is in
    /// [`DeviceState::RegistrationStarted`].
    pub fn require_verification(&mut self) -> Result<(), AuthError> {
        self.transition(
            &[DeviceState::RegistrationStarted],
            DeviceState::VerificationRequired,
        )
    }

    /// Marks verification complete and moves the device to registered
    /// (§24.1).
    ///
    /// # Errors
    ///
    /// Returns [`AuthError::InvalidTransition`] unless the device is in
    /// [`DeviceState::VerificationRequired`].
    pub fn complete_verification(&mut self) -> Result<(), AuthError> {
        self.transition(
            &[DeviceState::VerificationRequired],
            DeviceState::Registered,
        )
    }

    /// Activates a registered device (§24.1).
    ///
    /// # Errors
    ///
    /// Returns [`AuthError::InvalidTransition`] unless the device is in
    /// [`DeviceState::Registered`].
    pub fn activate(&mut self) -> Result<(), AuthError> {
        self.transition(&[DeviceState::Registered], DeviceState::Active)
    }

    /// Suspends an active device (§27).
    ///
    /// # Errors
    ///
    /// Returns [`AuthError::InvalidTransition`] unless the device is in
    /// [`DeviceState::Active`].
    pub fn suspend(&mut self) -> Result<(), AuthError> {
        self.transition(&[DeviceState::Active], DeviceState::Suspended)
    }

    /// Suspended devices may be reinstated; revoked ones never are (§27, A.2).
    ///
    /// # Errors
    ///
    /// Returns [`AuthError::InvalidTransition`] unless the device is in
    /// [`DeviceState::Suspended`].
    pub fn reinstate(&mut self) -> Result<(), AuthError> {
        self.transition(&[DeviceState::Suspended], DeviceState::Active)
    }

    /// Atomic revocation with security-version increment (§27). Returns the
    /// post-increment version for dependent-session invalidation.
    ///
    /// # Errors
    ///
    /// Returns [`AuthError::InvalidTransition`] when the device is already in
    /// a terminal state (revoked, replaced, or retired), because a terminal
    /// record can never be revoked again.
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
    ///
    /// # Errors
    ///
    /// Returns [`AuthError::InvalidTransition`] when the device is already in
    /// a terminal state (revoked, replaced, or retired).
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
    /// The device record after revocation.
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

    fn at(seconds: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH
            .checked_add(Duration::from_secs(seconds))
            .unwrap_or(SystemTime::UNIX_EPOCH)
    }

    fn device() -> Result<Device, AuthError> {
        let now = at(500);
        Ok(Device::begin_registration(
            DeviceId::new("dev-01")?,
            UserId::new("user-01")?,
            ClientPlatform::Android,
            SecurityVersion(1),
            now,
        ))
    }

    #[test]
    fn lifecycle_follows_appendix_a2() -> Result<(), AuthError> {
        let mut d = device()?;
        assert_eq!(d.state, DeviceState::RegistrationStarted);
        d.require_verification()?;
        d.complete_verification()?;
        d.activate()?;
        d.suspend()?;
        d.reinstate()?;
        let bumped = d.revoke(at(510))?;
        assert_eq!(bumped, SecurityVersion(2));
        assert_eq!(d.state, DeviceState::Revoked);
        assert!(d.revoked_at.is_some());
        Ok(())
    }

    #[test]
    fn revoked_device_never_becomes_active() -> Result<(), AuthError> {
        let now = at(500);
        let mut d = device()?;
        d.require_verification()?;
        d.complete_verification()?;
        d.activate()?;
        d.revoke(now)?;
        assert!(d.activate().is_err());
        assert!(d.reinstate().is_err());
        assert!(matches!(d.revoke(now), Err(AuthError::InvalidTransition)));
        Ok(())
    }

    #[test]
    fn registration_cannot_skip_verification() -> Result<(), AuthError> {
        let mut d = device()?;
        assert!(matches!(d.activate(), Err(AuthError::InvalidTransition)));
        assert!(matches!(
            d.complete_verification(),
            Err(AuthError::InvalidTransition)
        ));
        Ok(())
    }

    #[test]
    fn replacement_retires_old_without_reviving() -> Result<(), AuthError> {
        let now = at(500);
        let mut old = device()?;
        old.require_verification()?;
        old.complete_verification()?;
        old.activate()?;
        old.retire_for_replacement(now)?;
        assert_eq!(old.state, DeviceState::Retired);
        let mut fresh = device()?;
        fresh.replacement_of = Some(old.id.clone());
        assert_eq!(fresh.state, DeviceState::RegistrationStarted);
        Ok(())
    }
}
