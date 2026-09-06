use crate::AppConfig;
use sha2::{Digest, Sha256};
/// SHA-256 over an explicitly canonical non-secret representation.
pub fn config_fingerprint(c: &AppConfig) -> String {
    let data = format!(
        "v={}\\nenv={}\\nservice={}\\nversion={}\\nbind={}\\nbody={}\\ndb_host={}\\ndb_port={}\\ndb_name={}\\ndb_user={}\\ndb_ref={}\\npool_min={}\\npool_max={}\\notel={:?}\\nsample={}\\nlog={}\\nqueue={}",
        c.config_schema_version,
        c.environment,
        c.service_name,
        c.service_version,
        c.bind_address,
        c.max_request_body_bytes,
        c.db_host,
        c.db_port,
        c.db_name,
        c.db_user,
        c.db_password_ref,
        c.db_pool_min,
        c.db_pool_max,
        c.otel_endpoint,
        c.trace_sample_ratio,
        c.log_level,
        c.otel_max_queue
    );
    format!("sha256:{:x}", Sha256::digest(data.as_bytes()))
}
