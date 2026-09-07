//! Authentication and independently revocable security subjects.
//!
//! Phase 3 specification: owns authentication mechanisms, session lifecycle,
//! refresh rotation, MFA enrollment and verification, device identity,
//! password hashing, recovery, PKCE, JWT/JWKS validation, rate limiting,
//! offline capability, and the security context (§15, §32, §33).
//!
//! This crate deliberately provides protocol-neutral state machines and ports.
//! OIDC, password hashing, TOTP, and WebAuthn protocol processing belong to
//! maintained adapters selected at deployment time (§6.1); tenant membership
//! and permission checks belong to later phases (§1.2, §69).
#![forbid(unsafe_code)]

pub mod device;
pub mod error;
pub mod id;
pub mod login;
pub mod mfa;
pub mod offline;
pub mod password;
pub mod pkce;
pub mod principal;
pub mod provider;
pub mod ratelimit;
pub mod refresh;
pub mod reset;
pub mod session;
pub mod token;

pub use device::{Device, DeviceRevocationEffect, DeviceState};
pub use error::{AuthError, CredentialMaterial};
pub use id::{
    Assurance, AuditEventId, AuthenticationMethod, ChallengeId, ClientPlatform, DeviceId,
    InstallationId, MfaAuthenticatorId, RecoveryCodeId, RefreshCredentialId, RefreshFamilyId,
    ResetArtifactId, SecurityVersion, SessionClass, SessionId, UserId, base64url, hex,
};
pub use login::{InternalLoginOutcome, PublicLoginOutcome};
pub use mfa::{
    Challenge, ChallengePurpose, MfaAuthenticator, MfaAuthenticatorState, MfaKind, MfaState,
    RecoveryCodeRecord, TestTotpVerifier, TotpPolicy, TotpVerifier,
};
pub use offline::{OfflineGrant, OfflineOperation};
pub use password::{
    PasswordHasher, PasswordPolicy, PasswordVerifierRecord, TestPasswordHasher, needs_rehash,
};
pub use pkce::{AuthorizationTransaction, AuthorizationTransactionLog, PkceVerifier};
pub use principal::{
    AuthenticatedPrincipal, DeviceSecurityState, SecurityContext, SessionSecurityState,
};
pub use provider::{ExternalIdentity, IdentityProviderPort, TestIdentityProvider, TokenSet};
pub use ratelimit::{AbuseClass, RateLimitDecision, RateLimitRule, RateLimiter};
pub use refresh::{
    RefreshCredential, RefreshFamily, RefreshLedger, RefreshState, RotationOutcome, hash_raw,
};
pub use reset::{PasswordResetArtifact, ResetArtifactState};
pub use session::{
    RevocationScope, RevocationTrigger, Session, SessionLifetime, SessionPolicySet, SessionState,
};
pub use token::{
    JwksCache, JwksCachePolicy, KeyResolution, SignatureVerifier, TokenAlgorithm, TokenClaims,
    TokenUse, TokenValidationPolicy, ValidatedToken, validate_token,
};
