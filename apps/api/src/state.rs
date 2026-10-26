//! Immutable runtime dependencies shared with request handlers.
//!
//! Constructed only by [`crate::bootstrap`] after successful startup
//! validation. Everything here is safe to share: no secret values, only
//! references, fingerprints, and bounded operational state.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use sitolo_api::{AppError, authenticate_request, extract_bearer, map_auth_error};
use sitolo_application::TenancyService;
use sitolo_auth::{IdentityProviderPort, SecurityContext, SessionId};
use sitolo_config::DatabaseTarget;
use sitolo_observability::RequestId;
use sitolo_observability::TelemetryBuffer;
use sitolo_persistence::{PgRuntimePool, RequestIdentityAuthority};
use std::time::SystemTime;

const IDENTITY_READINESS_TIMEOUT: Duration = Duration::from_millis(900);

/// Immutable shared application state.
#[derive(Clone)]
pub struct AppState {
    service_name: String,
    service_version: String,
    config_fingerprint: String,
    db_target: DatabaseTarget,
    telemetry: Arc<Mutex<TelemetryBuffer>>,
    identity_provider: Option<Arc<dyn IdentityProviderPort>>,
    identity_authority: Option<Arc<dyn RequestIdentityAuthority>>,
    tenancy_service: Option<Arc<TenancyService>>,
    runtime_database: Option<PgRuntimePool>,
    runtime_ready: bool,
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Trait-object dependencies and the telemetry buffer are represented by
        // presence flags only: their internals must never reach a log line.
        formatter
            .debug_struct("AppState")
            .field("service_name", &self.service_name)
            .field("service_version", &self.service_version)
            .field("config_fingerprint", &self.config_fingerprint)
            .field("db_target", &self.db_target)
            .field("has_identity_provider", &self.identity_provider.is_some())
            .field("has_identity_authority", &self.identity_authority.is_some())
            .field("has_tenancy_service", &self.tenancy_service.is_some())
            .field("has_runtime_database", &self.runtime_database.is_some())
            .field("runtime_ready", &self.runtime_ready)
            .finish_non_exhaustive()
    }
}

impl AppState {
    /// Creates state with no optional runtime dependency composed. Every
    /// dependency must be installed explicitly by the composition root.
    #[must_use]
    pub fn new(
        service_name: String,
        service_version: String,
        config_fingerprint: String,
        db_target: DatabaseTarget,
        telemetry: Arc<Mutex<TelemetryBuffer>>,
    ) -> Self {
        AppState {
            service_name,
            service_version,
            config_fingerprint,
            db_target,
            telemetry,
            identity_provider: None,
            identity_authority: None,
            tenancy_service: None,
            runtime_database: None,
            runtime_ready: false,
        }
    }

    /// Adds the verified identity provider and persistent session authority used by
    /// the live HTTP authentication path. Production composition must supply real
    /// adapters; test providers must never be wired into a served production runtime.
    #[must_use]
    pub fn with_request_identity(
        mut self,
        identity_provider: Arc<dyn IdentityProviderPort>,
        identity_authority: Arc<dyn RequestIdentityAuthority>,
    ) -> Self {
        self.identity_provider = Some(identity_provider);
        self.identity_authority = Some(identity_authority);
        self
    }

    /// Adds the persistent identity/session authority while the OIDC provider
    /// is being separately composed. Authentication remains unavailable until
    /// both are installed.
    #[must_use]
    pub fn with_identity_authority(
        mut self,
        identity_authority: Arc<dyn RequestIdentityAuthority>,
    ) -> Self {
        self.identity_authority = Some(identity_authority);
        self
    }

    /// Adds the configured external identity provider independently of the
    /// persistent session authority.
    #[must_use]
    pub fn with_identity_provider(
        mut self,
        identity_provider: Arc<dyn IdentityProviderPort>,
    ) -> Self {
        self.identity_provider = Some(identity_provider);
        self
    }

    /// Adds the durable tenancy command service to an explicitly composed
    /// runtime. The production bootstrap must provide `new_postgres`; no
    /// in-memory default is installed here.
    ///
    /// # Errors
    /// This builder performs no I/O and cannot fail.
    ///
    /// # Panics
    /// This builder does not panic.
    #[must_use]
    pub fn with_tenancy_service(mut self, tenancy_service: Arc<TenancyService>) -> Self {
        self.tenancy_service = Some(tenancy_service);
        self
    }

    /// Returns the configured tenancy service, if one was explicitly composed.
    ///
    /// # Errors
    /// This accessor has no error result.
    ///
    /// # Panics
    /// This accessor does not panic.
    #[must_use]
    pub fn tenancy_service(&self) -> Option<&TenancyService> {
        self.tenancy_service.as_deref()
    }

    /// Whether a durable tenancy command service has been composed.
    #[must_use]
    pub fn has_tenancy_service(&self) -> bool {
        self.tenancy_service.is_some()
    }

    /// Marks readiness only after the durable database, identity, tenancy,
    /// and command dependencies have all been composed and probed.
    #[must_use]
    pub fn with_runtime_ready(mut self) -> Self {
        self.runtime_ready = true;
        self
    }

    /// Adds the verified production pool used by dynamic readiness probes.
    #[must_use]
    pub fn with_runtime_database(mut self, database: PgRuntimePool) -> Self {
        self.runtime_database = Some(database);
        self
    }

    /// Checks composition and live database/schema health before reporting ready.
    pub async fn runtime_ready(&self) -> bool {
        if !self.runtime_ready || !self.has_request_identity() || !self.has_tenancy_service() {
            return false;
        }
        let (Some(database), Some(identity_provider)) = (
            self.runtime_database.as_ref(),
            self.identity_provider.as_deref(),
        ) else {
            return false;
        };
        let (database_result, identity_ready) = tokio::join!(
            database.probe(),
            identity_provider_ready(identity_provider, IDENTITY_READINESS_TIMEOUT),
        );
        database_result.is_ok() && identity_ready
    }

    /// Whether both the external identity provider and persistent session
    /// authority have been installed into the request path.
    #[must_use]
    pub fn has_request_identity(&self) -> bool {
        self.identity_provider.is_some() && self.identity_authority.is_some()
    }

    /// Validates the bearer, exact linked identity and owner-bound Sitolo session.
    ///
    /// If the authentication dependencies are not composed, malformed or missing
    /// credentials still fail as authentication errors; a structurally valid request
    /// fails as dependency unavailable. Callers must perform tenant authorization
    /// before invoking protected commands.
    ///
    /// # Errors
    /// Returns [`AppError::Authentication`] when the credential is missing,
    /// malformed, or fails provider/session validation, and
    /// [`AppError::DependencyUnavailable`] when the authentication dependencies
    /// were never composed. No variant carries credential material.
    pub async fn authenticate_request(
        &self,
        authorization: Option<&str>,
        session_selector: Option<&str>,
    ) -> Result<SecurityContext, AppError> {
        let Some(session_selector) = session_selector else {
            return Err(AppError::Authentication);
        };
        let session_id =
            SessionId::new(session_selector.to_owned()).map_err(|error| map_auth_error(&error))?;
        // The bearer credential is validated for shape and then dropped: this
        // state never retains request credentials.
        drop(extract_bearer(authorization)?);

        let (Some(identity_provider), Some(identity_authority)) =
            (&self.identity_provider, &self.identity_authority)
        else {
            return Err(AppError::DependencyUnavailable);
        };

        authenticate_request(
            authorization,
            Some(session_id.as_str()),
            identity_provider.as_ref(),
            identity_authority.as_ref(),
            SystemTime::now(),
            RequestId::new_server(),
            None,
        )
        .await
    }

    /// Revokes the session established by this request's trusted security
    /// context. The caller cannot choose another session or user identifier.
    ///
    /// # Errors
    /// Returns [`AppError::DependencyUnavailable`] when the identity authority
    /// is not composed, and an authentication error when revocation or its
    /// required durable audit write fails.
    pub async fn logout_current_session(
        &self,
        security_context: &SecurityContext,
    ) -> Result<(), AppError> {
        let Some(identity_authority) = self.identity_authority.as_deref() else {
            return Err(AppError::DependencyUnavailable);
        };
        identity_authority
            .logout_session_for_user(
                &security_context.principal.session_id,
                &security_context.principal.subject_id,
                SystemTime::now(),
            )
            .await
            .map_err(|error| map_auth_error(&error))
    }

    /// Configured service name reported by the liveness probe.
    #[must_use]
    pub fn service_name(&self) -> &str {
        &self.service_name
    }

    /// Configured service version reported by the liveness probe.
    #[must_use]
    pub fn service_version(&self) -> &str {
        &self.service_version
    }

    /// Safe operational attribute; suitable for low-cardinality telemetry.
    #[must_use]
    pub fn config_fingerprint(&self) -> &str {
        &self.config_fingerprint
    }

    /// Non-secret database identity for diagnostics.
    #[must_use]
    pub fn db_target(&self) -> &DatabaseTarget {
        &self.db_target
    }

    /// Shared bounded telemetry buffer.
    #[must_use]
    pub fn telemetry(&self) -> &Arc<Mutex<TelemetryBuffer>> {
        &self.telemetry
    }
}

async fn identity_provider_ready(provider: &dyn IdentityProviderPort, timeout: Duration) -> bool {
    tokio::time::timeout(timeout, provider.readiness_probe())
        .await
        .is_ok_and(|result| result.is_ok())
}

#[cfg(test)]
mod tests {
    use super::identity_provider_ready;
    use sitolo_auth::{AuthError, ExternalIdentity, IdentityProviderPort, TokenSet};
    use sitolo_security::SecretValue;
    use std::time::Duration;

    #[derive(Clone, Copy)]
    enum ProbeMode {
        Ready,
        Unavailable,
        Pending,
    }

    struct ProbeProvider(ProbeMode);

    #[async_trait::async_trait]
    impl IdentityProviderPort for ProbeProvider {
        async fn readiness_probe(&self) -> Result<(), AuthError> {
            match self.0 {
                ProbeMode::Ready => Ok(()),
                ProbeMode::Unavailable => Err(AuthError::IdentityProviderUnavailable),
                ProbeMode::Pending => std::future::pending().await,
            }
        }

        async fn validate_assertion(
            &self,
            _assertion: &str,
        ) -> Result<ExternalIdentity, AuthError> {
            Err(AuthError::IdentityProviderUnavailable)
        }

        async fn exchange_code(
            &self,
            _code: SecretValue,
            _pkce_verifier: &str,
        ) -> Result<TokenSet, AuthError> {
            Err(AuthError::IdentityProviderUnavailable)
        }
    }

    #[tokio::test]
    async fn readiness_fails_closed_on_provider_error_and_timeout() {
        assert!(
            identity_provider_ready(&ProbeProvider(ProbeMode::Ready), Duration::from_millis(5))
                .await
        );
        assert!(
            !identity_provider_ready(
                &ProbeProvider(ProbeMode::Unavailable),
                Duration::from_millis(5),
            )
            .await
        );
        assert!(
            !identity_provider_ready(&ProbeProvider(ProbeMode::Pending), Duration::from_millis(5),)
                .await
        );
    }
}
