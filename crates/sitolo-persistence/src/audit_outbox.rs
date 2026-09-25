//! Phase 4 Part 8 Audit and Outbox persistence.
//!
//! Provides transactional audit recording and transactional outbox event insertion
//! along with outbox relay worker claiming, retry management, and quarantine transitions.

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use sitolo_audit::IamAuditEvent;
use sitolo_events::{OutboxEvent, OutboxEventId, OutboxStatus, RetryClassification};
use sqlx::{Postgres, Row, Transaction};

use crate::postgres::PgAuthorityError;

fn system_time_to_micros(time: SystemTime) -> i64 {
    let duration = time
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    duration.as_micros() as i64
}

fn opt_system_time_to_micros(time: Option<SystemTime>) -> Option<i64> {
    time.map(system_time_to_micros)
}

fn micros_to_system_time(micros: i64) -> SystemTime {
    if micros <= 0 {
        SystemTime::UNIX_EPOCH
    } else {
        SystemTime::UNIX_EPOCH + Duration::from_micros(micros as u64)
    }
}

fn opt_micros_to_system_time(micros: Option<i64>) -> Option<SystemTime> {
    micros.map(micros_to_system_time)
}

/// Audit and Outbox Store trait for transactional operations and worker claiming.
#[async_trait]
pub trait AuditOutboxStore: Send + Sync {
    async fn record_iam_audit_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &IamAuditEvent,
    ) -> Result<(), PgAuthorityError>;

    async fn enqueue_outbox_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &OutboxEvent,
    ) -> Result<(), PgAuthorityError>;

    async fn claim_outbox_events(
        &self,
        batch_size: u32,
        lease_duration: Duration,
        now: SystemTime,
    ) -> Result<Vec<OutboxEvent>, PgAuthorityError>;

    async fn mark_published(
        &self,
        event_id: &OutboxEventId,
        claim_token: &str,
        now: SystemTime,
    ) -> Result<(), PgAuthorityError>;

    async fn mark_failed_or_quarantined(
        &self,
        event_id: &OutboxEventId,
        claim_token: &str,
        retry: RetryClassification,
        error_class: &str,
        now: SystemTime,
        max_attempts: u32,
        backoff_duration: Duration,
    ) -> Result<OutboxStatus, PgAuthorityError>;
}

/// PostgreSQL implementation of Audit and Outbox persistence.
#[derive(Clone)]
pub struct PgAuditOutboxStore {
    runtime_pool: sqlx::PgPool,
    worker_pool: sqlx::PgPool,
}

impl PgAuditOutboxStore {
    pub fn new(runtime_pool: sqlx::PgPool, worker_pool: sqlx::PgPool) -> Self {
        Self { runtime_pool, worker_pool }
    }
}

#[async_trait]
impl AuditOutboxStore for PgAuditOutboxStore {
    async fn record_iam_audit_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &IamAuditEvent,
    ) -> Result<(), PgAuthorityError> {
        let result_str = match event.result {
            sitolo_audit::EventResult::Success => "SUCCESS",
            sitolo_audit::EventResult::Failure => "FAILURE",
        };
        let assurance_str = event.assurance_level.map(|a| a.as_str());
        let occurred_micros = system_time_to_micros(event.occurred_at);

        sqlx::query(
            "INSERT INTO iam_audit_records (
                event_id, event_name, event_version, occurred_at,
                organization_id, branch_id, actor_subject_ref, actor_membership_ref, actor_device_ref,
                request_id, trace_id, target_type, target_ref, action, result, reason_class,
                assurance_level, source, metadata
            ) VALUES (
                $1, $2, $3, to_timestamp($4::double precision / 1000000.0),
                $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19
            )"
        )
        .bind(event.event_id.as_str())
        .bind(event.event_name.as_str())
        .bind(event.event_version as i32)
        .bind(occurred_micros)
        .bind(&event.organization_id)
        .bind(&event.branch_id)
        .bind(&event.actor_subject_ref)
        .bind(&event.actor_membership_ref)
        .bind(&event.actor_device_ref)
        .bind(&event.request_id)
        .bind(&event.trace_id)
        .bind(&event.target_type)
        .bind(&event.target_ref)
        .bind(&event.action)
        .bind(result_str)
        .bind(&event.reason_class)
        .bind(assurance_str)
        .bind(&event.source)
        .bind(&event.metadata)
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    async fn enqueue_outbox_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &OutboxEvent,
    ) -> Result<(), PgAuthorityError> {
        let occurred_micros = system_time_to_micros(event.occurred_at);
        let available_micros = system_time_to_micros(event.available_at);
        let locked_micros = opt_system_time_to_micros(event.locked_at);
        let published_micros = opt_system_time_to_micros(event.published_at);

        sqlx::query(
            "INSERT INTO outbox_events (
                event_id, aggregate_type, aggregate_id, aggregate_sequence, event_name, event_version,
                organization_id, branch_id, occurred_at, payload, status, available_at,
                attempt_count, locked_at, published_at, last_error_class, deduplication_key, schema_version, claim_token
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8,
                to_timestamp($9::double precision / 1000000.0), $10, 'PENDING',
                to_timestamp($11::double precision / 1000000.0), 0,
                NULL, NULL,
                NULL, $12, $13, NULL
            ) ON CONFLICT (aggregate_type, aggregate_id, aggregate_sequence) DO NOTHING"
        )
        .bind(event.event_id.as_str())
        .bind(&event.aggregate_type)
        .bind(&event.aggregate_id)
        .bind(event.aggregate_sequence)
        .bind(&event.event_name)
        .bind(event.event_version as i32)
        .bind(&event.organization_id)
        .bind(&event.branch_id)
        .bind(occurred_micros)
        .bind(&event.payload)
        .bind(available_micros)
        .bind(&event.deduplication_key)
        .bind(event.schema_version as i32)
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    async fn claim_outbox_events(
        &self,
        batch_size: u32,
        lease_duration: Duration,
        now: SystemTime,
    ) -> Result<Vec<OutboxEvent>, PgAuthorityError> {
        let lease_sec = lease_duration.as_secs() as i64;
        let now_micros = system_time_to_micros(now);

        let rows = sqlx::query(
            "UPDATE outbox_events
             SET status = 'CLAIMED',
                 locked_at = to_timestamp($1::double precision / 1000000.0),
                 attempt_count = attempt_count + 1,
                 claim_token = gen_random_uuid()::text
             WHERE event_id IN (
                 SELECT event_id
                 FROM outbox_events
                 WHERE (status = 'PENDING' AND available_at <= to_timestamp($1::double precision / 1000000.0))
                    OR (status = 'CLAIMED' AND locked_at < to_timestamp($1::double precision / 1000000.0) - ($2 || ' seconds')::interval)
                 ORDER BY aggregate_sequence ASC
                 LIMIT $3
                 FOR UPDATE SKIP LOCKED
             )
             RETURNING event_id, aggregate_type, aggregate_id, event_name, event_version,
                       organization_id, branch_id,
                       (EXTRACT(EPOCH FROM occurred_at) * 1000000)::bigint AS occurred_micros,
                       payload, status,
                       (EXTRACT(EPOCH FROM available_at) * 1000000)::bigint AS available_micros,
                       attempt_count,
                       (EXTRACT(EPOCH FROM locked_at) * 1000000)::bigint AS locked_micros,
                       (EXTRACT(EPOCH FROM published_at) * 1000000)::bigint AS published_micros,
                       last_error_class, deduplication_key, schema_version, claim_token, aggregate_sequence"
        )
        .bind(now_micros)
        .bind(lease_sec)
        .bind(batch_size as i64)
        .fetch_all(&self.worker_pool)
        .await?;

        let mut events = Vec::new();
        for row in rows {
            let status_str: String = row.get("status");
            let status = match status_str.as_str() {
                "CLAIMED" => OutboxStatus::Claimed,
                "PUBLISHED" => OutboxStatus::Published,
                "QUARANTINED" => OutboxStatus::Quarantined,
                _ => OutboxStatus::Pending,
            };

            let id_str: String = row.get("event_id");
            let event_id = OutboxEventId::new(id_str).map_err(|_| PgAuthorityError::Sqlx)?;

            let occurred_micros: i64 = row.get("occurred_micros");
            let available_micros: i64 = row.get("available_micros");
            let locked_micros: Option<i64> = row.get("locked_micros");
            let published_micros: Option<i64> = row.get("published_micros");

            events.push(OutboxEvent {
                event_id,
                aggregate_type: row.get("aggregate_type"),
                aggregate_id: row.get("aggregate_id"),
                event_name: row.get("event_name"),
                event_version: row.get::<i32, _>("event_version") as u32,
                organization_id: row.get("organization_id"),
                branch_id: row.get("branch_id"),
                occurred_at: micros_to_system_time(occurred_micros),
                payload: row.get("payload"),
                status,
                available_at: micros_to_system_time(available_micros),
                attempt_count: row.get::<i32, _>("attempt_count") as u32,
                locked_at: opt_micros_to_system_time(locked_micros),
                published_at: opt_micros_to_system_time(published_micros),
                last_error_class: row.get("last_error_class"),
                deduplication_key: row.get("deduplication_key"),
                schema_version: row.get::<i32, _>("schema_version") as u32,
                claim_token: row.get("claim_token"),
                aggregate_sequence: row.get::<i64, _>("aggregate_sequence"),
            });
        }

        Ok(events)
    }

    async fn mark_published(
        &self,
        event_id: &OutboxEventId,
        now: SystemTime,
    ) -> Result<(), PgAuthorityError> {
        let now_micros = system_time_to_micros(now);

        let res = sqlx::query(
            "UPDATE outbox_events
             SET status = 'PUBLISHED',
                 published_at = to_timestamp($1::double precision / 1000000.0),
                 locked_at = NULL
             WHERE event_id = $2 AND claim_token = $3",
        )
        .bind(now_micros)
        .bind(event_id.as_str())
        .bind(claim_token)
        .execute(&self.worker_pool)
        .await?;

        if res.rows_affected() == 0 {
            Err(PgAuthorityError::NotFoundOrDenied)
        } else {
            Ok(())
        }
    }

    async fn mark_failed_or_quarantined(
        &self,
        event_id: &OutboxEventId,
        retry: RetryClassification,
        error_class: &str,
        now: SystemTime,
        max_attempts: u32,
        backoff_duration: Duration,
    ) -> Result<OutboxStatus, PgAuthorityError> {
        let current_attempts: i32 =
            sqlx::query_scalar("SELECT attempt_count FROM outbox_events WHERE event_id = $1")
                .bind(event_id.as_str())
                .fetch_one(&self.worker_pool)
                .await?;

        let quarantine =
            retry == RetryClassification::Terminal || (current_attempts as u32) >= max_attempts;

        if quarantine {
            sqlx::query(
                "UPDATE outbox_events
                 SET status = 'QUARANTINED',
                     last_error_class = $1,
                     locked_at = NULL
                 WHERE event_id = $2 AND claim_token = $3",
            )
            .bind(error_class)
            .bind(event_id.as_str())
            .bind(claim_token)
            .execute(&self.worker_pool)
            .await?;

            Ok(OutboxStatus::Quarantined)
        } else {
            let next_available_micros = system_time_to_micros(now + backoff_duration);
            sqlx::query(
                "UPDATE outbox_events
                 SET status = 'PENDING',
                     available_at = to_timestamp($1::double precision / 1000000.0),
                     last_error_class = $2,
                     locked_at = NULL
                 WHERE event_id = $3 AND claim_token = $4",
            )
            .bind(next_available_micros)
            .bind(error_class)
            .bind(event_id.as_str())
            .bind(claim_token)
            .execute(&self.worker_pool)
            .await?;

            Ok(OutboxStatus::Pending)
        }
    }
}

/// In-memory Audit and Outbox database for unit testing.
#[derive(Default)]
pub struct AuditOutboxDatabase {
    audit_records: Mutex<Vec<IamAuditEvent>>,
    outbox_records: Mutex<BTreeMap<String, OutboxEvent>>,
}

impl AuditOutboxDatabase {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_iam_audit(&self, event: IamAuditEvent) {
        let mut list = self.audit_records.lock().unwrap();
        list.push(event);
    }

    pub fn enqueue_outbox(&self, event: OutboxEvent) -> Result<(), String> {
        let mut map = self.outbox_records.lock().unwrap();
        if map.contains_key(event.event_id.as_str()) {
            return Err("Duplicate outbox event ID".into());
        }
        for existing in map.values() {
            if existing.deduplication_key == event.deduplication_key {
                return Err("Duplicate deduplication key".into());
            }
        }
        map.insert(event.event_id.as_str().to_string(), event);
        Ok(())
    }

    pub fn audit_records(&self) -> Vec<IamAuditEvent> {
        self.audit_records.lock().unwrap().clone()
    }

    pub fn outbox_records(&self) -> Vec<OutboxEvent> {
        self.outbox_records
            .lock()
            .unwrap()
            .values()
            .cloned()
            .collect()
    }
}

#[async_trait]
impl AuditOutboxStore for AuditOutboxDatabase {
    async fn record_iam_audit_tx(
        &self,
        _tx: &mut Transaction<'_, Postgres>,
        event: &IamAuditEvent,
    ) -> Result<(), PgAuthorityError> {
        self.record_iam_audit(event.clone());
        Ok(())
    }

    async fn enqueue_outbox_tx(
        &self,
        _tx: &mut Transaction<'_, Postgres>,
        event: &OutboxEvent,
    ) -> Result<(), PgAuthorityError> {
        self.enqueue_outbox(event.clone())
            .map_err(|_| PgAuthorityError::NotFoundOrDenied)
    }

    async fn claim_outbox_events(
        &self,
        batch_size: u32,
        lease_duration: Duration,
        now: SystemTime,
    ) -> Result<Vec<OutboxEvent>, PgAuthorityError> {
        let mut map = self.outbox_records.lock().unwrap();
        let mut claimed = Vec::new();

        for event in map.values_mut() {
            if claimed.len() as u32 >= batch_size {
                break;
            }
            let is_pending = event.status == OutboxStatus::Pending && event.available_at <= now;
            let is_expired_lease = event.status == OutboxStatus::Claimed
                && event
                    .locked_at
                    .is_none_or(|locked| locked + lease_duration < now);

            if is_pending || is_expired_lease {
                event.status = OutboxStatus::Claimed;
                event.locked_at = Some(now);
                event.attempt_count += 1;
                claimed.push(event.clone());
            }
        }

        Ok(claimed)
    }

    async fn mark_published(
        &self,
        event_id: &OutboxEventId,
        now: SystemTime,
    ) -> Result<(), PgAuthorityError> {
        let mut map = self.outbox_records.lock().unwrap();
        let event = map
            .get_mut(event_id.as_str())
            .ok_or(PgAuthorityError::NotFoundOrDenied)?;
        event.status = OutboxStatus::Published;
        event.published_at = Some(now);
        event.locked_at = None;
        Ok(())
    }

    async fn mark_failed_or_quarantined(
        &self,
        event_id: &OutboxEventId,
        retry: RetryClassification,
        error_class: &str,
        now: SystemTime,
        max_attempts: u32,
        backoff_duration: Duration,
    ) -> Result<OutboxStatus, PgAuthorityError> {
        let mut map = self.outbox_records.lock().unwrap();
        let event = map
            .get_mut(event_id.as_str())
            .ok_or(PgAuthorityError::NotFoundOrDenied)?;

        let quarantine =
            retry == RetryClassification::Terminal || event.attempt_count >= max_attempts;

        if quarantine {
            event.status = OutboxStatus::Quarantined;
            event.last_error_class = Some(error_class.to_string());
            event.locked_at = None;
            Ok(OutboxStatus::Quarantined)
        } else {
            event.status = OutboxStatus::Pending;
            event.available_at = now + backoff_duration;
            event.last_error_class = Some(error_class.to_string());
            event.locked_at = None;
            Ok(OutboxStatus::Pending)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sitolo_audit::{EventResult, IamAuditEventName};
    use sitolo_auth::{Assurance, AuditEventId};

    #[tokio::test]
    async fn audit_outbox_database_records_and_claims() {
        let db = AuditOutboxDatabase::new();
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);

        let audit = IamAuditEvent {
            event_id: AuditEventId::new("evt-audit-1").unwrap(),
            event_name: IamAuditEventName::OrganizationCreated,
            event_version: 1,
            occurred_at: now,
            organization_id: Some("org-1".into()),
            branch_id: None,
            actor_subject_ref: Some("user-1".into()),
            actor_membership_ref: None,
            actor_device_ref: None,
            request_id: Some("req-1".into()),
            trace_id: None,
            target_type: Some("organization".into()),
            target_ref: Some("org-1".into()),
            action: "iam.organization.created".into(),
            result: EventResult::Success,
            reason_class: None,
            assurance_level: Some(Assurance::A1),
            source: "api".into(),
            metadata: None,
        };
        db.record_iam_audit(audit);
        assert_eq!(db.audit_records().len(), 1);

        let outbox = OutboxEvent {
            event_id: OutboxEventId::new("evt-outbox-1").unwrap(),
            aggregate_type: "organization".into(),
            aggregate_id: "org-1".into(),
            event_name: "iam.organization.created".into(),
            event_version: 1,
            organization_id: Some("org-1".into()),
            branch_id: None,
            occurred_at: now,
            payload: "{}".into(),
            status: OutboxStatus::Pending,
            available_at: now,
            attempt_count: 0,
            locked_at: None,
            published_at: None,
            last_error_class: None,
            deduplication_key: "dedup-1".into(),
            schema_version: 1,
        };
        db.enqueue_outbox(outbox).unwrap();

        let claimed = db
            .claim_outbox_events(10, Duration::from_secs(30), now)
            .await
            .unwrap();
        assert_eq!(claimed.len(), 1);
        assert_eq!(claimed[0].status, OutboxStatus::Claimed);
        assert_eq!(claimed[0].attempt_count, 1);

        db.mark_published(&claimed[0].event_id, claimed[0].claim_token.as_deref().unwrap_or(""), now).await.unwrap();
        let records = db.outbox_records();
        assert_eq!(records[0].status, OutboxStatus::Published);
    }

    #[test]
    fn outbox_deduplication_key_prevents_duplicate_enqueue() {
        let db = AuditOutboxDatabase::new();
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);

        let event1 = OutboxEvent {
            event_id: OutboxEventId::new("evt-1").unwrap(),
            aggregate_type: "membership".into(),
            aggregate_id: "mem-1".into(),
            event_name: "iam.membership.invited".into(),
            event_version: 1,
            organization_id: Some("org-1".into()),
            branch_id: None,
            occurred_at: now,
            payload: "{}".into(),
            status: OutboxStatus::Pending,
            available_at: now,
            attempt_count: 0,
            locked_at: None,
            published_at: None,
            last_error_class: None,
            deduplication_key: "same-key".into(),
            schema_version: 1,
        };
        db.enqueue_outbox(event1).unwrap();

        let mut event2 = OutboxEvent {
            event_id: OutboxEventId::new("evt-2").unwrap(),
            aggregate_type: "membership".into(),
            aggregate_id: "mem-1".into(),
            event_name: "iam.membership.invited".into(),
            event_version: 1,
            organization_id: Some("org-1".into()),
            branch_id: None,
            occurred_at: now,
            payload: "{}".into(),
            status: OutboxStatus::Pending,
            available_at: now,
            attempt_count: 0,
            locked_at: None,
            published_at: None,
            last_error_class: None,
            deduplication_key: "same-key".into(),
            schema_version: 1,
        };

        assert!(db.enqueue_outbox(event2.clone()).is_err());
        event2.deduplication_key = "different-key".into();
        assert!(db.enqueue_outbox(event2).is_ok());
    }

    #[tokio::test]
    async fn quarantine_after_max_attempts() {
        let db = AuditOutboxDatabase::new();
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);

        let event = OutboxEvent {
            event_id: OutboxEventId::new("evt-fail").unwrap(),
            aggregate_type: "role".into(),
            aggregate_id: "role-1".into(),
            event_name: "iam.role.assigned".into(),
            event_version: 1,
            organization_id: Some("org-1".into()),
            branch_id: None,
            occurred_at: now,
            payload: "{}".into(),
            status: OutboxStatus::Pending,
            available_at: now,
            attempt_count: 0,
            locked_at: None,
            published_at: None,
            last_error_class: None,
            deduplication_key: "key-fail".into(),
            schema_version: 1,
        };
        db.enqueue_outbox(event.clone()).unwrap();

        let claimed = db
            .claim_outbox_events(1, Duration::from_secs(30), now)
            .await
            .unwrap();
        let status = db
            .mark_failed_or_quarantined(
                &claimed[0].event_id,
                RetryClassification::Retryable,
                "network_timeout",
                now,
                1, // max_attempts = 1
                Duration::from_secs(5),
            )
            .await
            .unwrap();

        assert_eq!(status, OutboxStatus::Quarantined);
    }
}
