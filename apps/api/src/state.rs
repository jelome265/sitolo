//! Immutable runtime dependencies shared with request handlers.
//!
//! Constructed only by [`crate::bootstrap`] after successful startup
//! validation. Everything here is safe to share: no secret values, only
//! references, fingerprints, and bounded operational state.

use std::sync::{Arc, Mutex};

use sitolo_application::TenancyService;
use sitolo_config::DatabaseTarget;
use sitolo_observability::TelemetryBuffer;
use sitolo_persistence::TenancyDatabase;

use crate::shutdown::Readiness;

/// Immutable shared application state.
#[derive(Clone)]
pub struct AppState {
    service_name: String,
    service_version: String,
    config_fingerprint: String,
    db_target: DatabaseTarget,
    telemetry: Arc<Mutex<TelemetryBuffer>>,
    tenancy_service: Arc<TenancyService>,
    readiness: Readiness,
}

impl AppState {
    pub fn new(
        service_name: String,
        service_version: String,
        config_fingerprint: String,
        db_target: DatabaseTarget,
        telemetry: Arc<Mutex<TelemetryBuffer>>,
    ) -> Self {
        let tenancy_db = Arc::new(TenancyDatabase::new());
        let tenancy_service = Arc::new(TenancyService::new(tenancy_db, Vec::new()));
        AppState {
            service_name,
            service_version,
            config_fingerprint,
            db_target,
            telemetry,
            tenancy_service,
            readiness: Readiness::new(),
        }
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

    pub fn tenancy_service(&self) -> &Arc<TenancyService> {
        &self.tenancy_service
    }

    /// The single authoritative process lifecycle signal. `/process/ready`
    /// consults this rather than returning an unconditional success
    /// response; `bootstrap.rs` publishes `Ready` once startup completes,
    /// and `serve.rs` publishes `Draining` the instant a shutdown signal
    /// arrives, before anything else happens.
    pub fn readiness(&self) -> &Readiness {
        &self.readiness
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shutdown::ReadinessState;

    fn test_state() -> AppState {
        AppState::new(
            "sitolo".into(),
            "test".into(),
            "fingerprint".into(),
            DatabaseTarget {
                host: "localhost".into(),
                port: 5432,
                database: "sitolo".into(),
                username: "sitolo".into(),
            },
            Arc::new(Mutex::new(TelemetryBuffer::new(8))),
        )
    }

    #[test]
    fn new_state_starts_initializing() {
        let state = test_state();
        assert_eq!(state.readiness().get_state(), ReadinessState::Initializing);
        assert!(!state.readiness().is_draining());
    }

    #[test]
    fn clone_shares_the_same_readiness_handle() {
        let state = test_state();
        let cloned = state.clone();
        cloned.readiness().mark_ready();
        // AppState is cloned per-request by Axum's `State` extractor; every
        // clone must observe the same lifecycle signal, not a private copy.
        assert_eq!(state.readiness().get_state(), ReadinessState::Ready);
    }

    #[test]
    fn accessors_return_constructed_values() {
        let state = test_state();
        assert_eq!(state.service_name(), "sitolo");
        assert_eq!(state.service_version(), "test");
        assert_eq!(state.config_fingerprint(), "fingerprint");
        assert_eq!(state.db_target().host, "localhost");
    }
}
