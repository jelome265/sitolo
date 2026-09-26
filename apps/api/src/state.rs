//! Immutable runtime dependencies shared with request handlers.
use std::sync::{Arc, Mutex};
use sitolo_application::TenancyService;
use sitolo_config::DatabaseTarget;
use sitolo_observability::TelemetryBuffer;
use sitolo_persistence::TenancyDatabase;
use crate::shutdown::Readiness;

#[derive(Clone)]
pub struct AppState {
    service_name: String, service_version: String, config_fingerprint: String,
    db_target: DatabaseTarget, telemetry: Arc<Mutex<TelemetryBuffer>>,
    tenancy_service: Arc<TenancyService>, readiness: Readiness,
}

impl AppState {
    pub fn new(
        service_name: String, service_version: String, config_fingerprint: String,
        db_target: DatabaseTarget, telemetry: Arc<Mutex<TelemetryBuffer>>,
    ) -> Self {
        let tenancy_db = Arc::new(TenancyDatabase::new());
        let tenancy_service = Arc::new(TenancyService::new(tenancy_db, Vec::new()));
        Self {
            service_name, service_version, config_fingerprint, db_target,
            telemetry, tenancy_service, readiness: Readiness::new(),
        }
    }
    pub fn service_name(&self) -> &str { &self.service_name }
    pub fn service_version(&self) -> &str { &self.service_version }
    pub fn config_fingerprint(&self) -> &str { &self.config_fingerprint }
    pub fn db_target(&self) -> &DatabaseTarget { &self.db_target }
    pub fn telemetry(&self) -> &Arc<Mutex<TelemetryBuffer>> { &self.telemetry }
    pub fn tenancy_service(&self) -> &Arc<TenancyService> { &self.tenancy_service }
    pub fn readiness(&self) -> &Readiness { &self.readiness }
}
