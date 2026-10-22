//! Immutable runtime dependencies shared with request handlers.
//!
//! Constructed only by [`crate::bootstrap`] after successful startup
//! validation. Everything here is safe to share: no secret values, only
//! references, fingerprints, and bounded operational state.

use std::sync::{Arc, Mutex};

use sitolo_api::{AppError, authenticate_request, extract_bearer, map_auth_error};
use sitolo_auth::{IdentityProviderPort, SecurityContext, SessionId};
use sitolo_config::DatabaseTarget;
use sitolo_observability::RequestId;
use sitolo_observability::TelemetryBuffer;
use sitolo_persistence::RequestIdentityAuthority;
use std::time::SystemTime;

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
    runtime_ready: bool,
}

impl AppState {
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

    /// Marks readiness only after the durable database, identity, tenancy,
    /// and command dependencies have all been composed and probed.
    #[must_use]
    pub fn with_runtime_ready(mut self) -> Self {
        self.runtime_ready = true;
        self
    }

    pub fn runtime_ready(&self) -> bool {
        self.runtime_ready
    }

    /// Validates the bearer, exact linked identity and owner-bound Sitolo session.
    ///
    /// If the authentication dependencies are not composed, malformed or missing
    /// credentials still fail as authentication errors; a structurally valid request
    /// fails as dependency unavailable. Callers must perform tenant authorization
    /// before invoking protected commands.
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
        let _credential = extract_bearer(authorization)?;
        drop(_credential);

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

    pub fn service_name(&self) -> &str {
        &self.service_name
    }

    pub fn service_version(&self) -> &str {
        &self.service_version
    }

    /// Safe operational attribute; suitable for low-cardinality telemetry.
    pub fn config_fingerprint(&self) -> &str {
        &self.config_fingerprint
    }

    /// Non-secret database identity for diagnostics.
    pub fn db_target(&self) -> &DatabaseTarget {
        &self.db_target
    }

    pub fn telemetry(&self) -> &Arc<Mutex<TelemetryBuffer>> {
        &self.telemetry
    }
}
