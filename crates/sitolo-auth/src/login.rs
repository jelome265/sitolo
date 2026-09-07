//! Login outcome and enumeration resistance.
//!
//! Phase 3 specification, §22. Public authentication flows must not reveal
//! whether an account exists. Internal outcomes are mapped to identical
//! public responses for invalid credential and unknown identifier.

/// Internal login outcome (§22).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InternalLoginOutcome {
    Authenticated,
    InvalidCredential,
    UnknownIdentifier,
    RateLimited,
    MfaRequired,
}

/// Public login outcome (§22).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublicLoginOutcome {
    Authenticated,
    GenericFailure,
    RateLimited,
    MfaRequired,
}

impl PublicLoginOutcome {
    /// Maps an internal outcome to a public outcome (§22). Invalid credential
    /// and unknown identifier both map to `GenericFailure` to prevent
    /// enumeration.
    #[must_use]
    pub fn from_internal(outcome: InternalLoginOutcome) -> Self {
        match outcome {
            InternalLoginOutcome::Authenticated => PublicLoginOutcome::Authenticated,
            InternalLoginOutcome::InvalidCredential => PublicLoginOutcome::GenericFailure,
            InternalLoginOutcome::UnknownIdentifier => PublicLoginOutcome::GenericFailure,
            InternalLoginOutcome::RateLimited => PublicLoginOutcome::RateLimited,
            InternalLoginOutcome::MfaRequired => PublicLoginOutcome::MfaRequired,
        }
    }

    /// The stable public code for this outcome (§22, Appendix B).
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            PublicLoginOutcome::Authenticated => "AUTHENTICATION_SUCCESS",
            PublicLoginOutcome::GenericFailure => "AUTHENTICATION_FAILED",
            PublicLoginOutcome::RateLimited => "AUTHENTICATION_RATE_LIMITED",
            PublicLoginOutcome::MfaRequired => "MFA_REQUIRED",
        }
    }

    /// The stable HTTP status for this outcome (§22, Appendix B).
    #[must_use]
    pub fn status(self) -> u16 {
        match self {
            PublicLoginOutcome::Authenticated => 200,
            PublicLoginOutcome::GenericFailure => 401,
            PublicLoginOutcome::RateLimited => 429,
            PublicLoginOutcome::MfaRequired => 403,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enumeration_resistance() {
        let invalid = PublicLoginOutcome::from_internal(InternalLoginOutcome::InvalidCredential);
        let unknown = PublicLoginOutcome::from_internal(InternalLoginOutcome::UnknownIdentifier);
        assert_eq!(invalid, unknown);
        assert_eq!(invalid.code(), "AUTHENTICATION_FAILED");
        assert_eq!(invalid.status(), 401);
    }

    #[test]
    fn rate_limited_is_distinct() {
        let rate = PublicLoginOutcome::from_internal(InternalLoginOutcome::RateLimited);
        assert_eq!(rate.code(), "AUTHENTICATION_RATE_LIMITED");
        assert_eq!(rate.status(), 429);
    }

    #[test]
    fn mfa_required_is_distinct() {
        let mfa = PublicLoginOutcome::from_internal(InternalLoginOutcome::MfaRequired);
        assert_eq!(mfa.code(), "MFA_REQUIRED");
        assert_eq!(mfa.status(), 403);
    }
}
