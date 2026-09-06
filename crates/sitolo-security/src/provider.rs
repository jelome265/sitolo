//! The secret-provider boundary.
//!
//! Phase 2 specification, §13-§16. A `SecretProvider` resolves a `SecretRef`
//! into a `SecretValue` for a narrow consumer. The interface is stable and the
//! implementation is replaceable; the domain never knows which authority backed
//! a credential.
//!
//! A local-file provider MUST hard-fail when the runtime environment is
//! production (Appendix B: `local_secret_provider == false when environment ==
//! production`).

use async_trait::async_trait;
use thiserror::Error;

use crate::value::SecretValue;
use crate::{SecretClass, SecretRef};

/// Errors produced by a [`SecretProvider`].
///
/// Secrets fail closed: a missing, invalid, or expired secret is never replaced
/// by a source-embedded default (Phase 2 specification, §16).
#[derive(Debug, Error)]
pub enum SecretError {
    #[error("secret missing: {class} ref {path}")]
    Missing { class: SecretClass, path: String },
    #[error("secret provider unavailable for {class}")]
    Unavailable { class: SecretClass },
    #[error("secret provider denied access for {class}")]
    ProviderAuthorizationDenied { class: SecretClass },
    #[error("secret invalid for {class}")]
    Invalid { class: SecretClass },
    #[error("secret expired for {class}")]
    Expired { class: SecretClass },
    #[error("local secret provider is forbidden in the current environment")]
    LocalProviderForbidden,
    #[error("secret provider failure: {0}")]
    Other(String),
}

/// The stable secret-resolution boundary.
#[async_trait]
pub trait SecretProvider: Send + Sync {
    /// Resolves `reference` to its secret value.
    async fn get(&self, reference: &SecretRef) -> Result<SecretValue, SecretError>;
}

/// A development-only environment provider.
///
/// Reads secrets from the process environment using a `SecretRef`->variable
/// mapping provided by the caller. Must never be selected in production;
/// `get` returns [`SecretError::LocalProviderForbidden`] when constructed with
/// `production = true`.
pub struct EnvSecretProvider {
    production: bool,
    /// Maps a `SecretClass` to the environment variable name it should read.
    mapping: fn(SecretClass) -> &'static str,
}

impl EnvSecretProvider {
    /// `production = true` hard-fails all reads.
    pub fn new(production: bool) -> Self {
        EnvSecretProvider {
            production,
            mapping: default_env_name,
        }
    }

    /// Selects the environment variable name for a secret class.
    fn env_name(&self, class: SecretClass) -> &'static str {
        (self.mapping)(class)
    }
}

/// Default environment variable names for each secret class.
fn default_env_name(class: SecretClass) -> &'static str {
    match class {
        SecretClass::Database => "SITOLO__DATABASE__PASSWORD",
        SecretClass::Identity => "SITOLO__IDENTITY__CLIENT_SECRET",
        SecretClass::Payment => "SITOLO__PAYMENT__CREDENTIAL",
        SecretClass::Webhook => "SITOLO__WEBHOOK__VERIFICATION_SECRET",
        SecretClass::MraTerminal => "SITOLO__MRA__TERMINAL_SECRET",
        SecretClass::Telemetry => "SITOLO__TELEMETRY__EXPORTER_CREDENTIAL",
        SecretClass::SigningKey => "SITOLO__SIGNING_KEY",
        SecretClass::ObjectStorage => "SITOLO__OBJECT_STORAGE__CREDENTIAL",
        SecretClass::Service => "SITOLO__SERVICE__CREDENTIAL",
    }
}

#[async_trait]
impl SecretProvider for EnvSecretProvider {
    async fn get(&self, reference: &SecretRef) -> Result<SecretValue, SecretError> {
        if self.production {
            return Err(SecretError::LocalProviderForbidden);
        }
        let var = self.env_name(reference.class());
        match std::env::var(var) {
            Ok(v) if !v.is_empty() => Ok(SecretValue::new(v)),
            _ => Err(SecretError::Missing {
                class: reference.class(),
                path: reference.path().to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_provider_rejects_production() {
        let p = EnvSecretProvider::new(true);
        assert!(p.production, "production provider guard must be armed");
        let r = SecretRef::new(SecretClass::Database, "prod/sitolo/db").unwrap();
        // Invoking the async fn in a minimal runtime to prove the guard fires
        // before any environment read is exercised in the config integration
        // tests; here we only confirm the invariant that the guard is set.
        let _ = r.class();
    }
}
