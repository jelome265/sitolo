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

/// Immutable shared application state.
pub struct AppState {
    service_name: String,
    service_version: String,
    config_fingerprint: String,
    db_target: DatabaseTarget,
    telemetry: Arc<Mutex<TelemetryBuffer>>,
    tenancy_service: Arc<TenancyService<TenancyDatabase>>,
}

impl AppState {
    pub fn new(
        service_name: String,
        service_version: String,
        config_fingerprint: String,
        db_target: DatabaseTarget,
        telemetry: Arc<Mutex<TelemetryBuffer>>,
        tenancy_service: Arc<TenancyService<TenancyDatabase>>,
    ) -> Self {
        AppState {
            service_name,
            service_version,
            config_fingerprint,
            db_target,
            telemetry,
            tenancy_service,
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

    pub fn tenancy_service(&self) -> &Arc<TenancyService<TenancyDatabase>> {
        &self.tenancy_service
    }
}
