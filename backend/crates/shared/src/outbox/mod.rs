//! Transactional domain-event outbox.
//!
//! Callers add the event through [`OutboxRepository::enqueue_on`] using the
//! same `PgConnection` as their domain mutation. The publisher claims rows
//! with `FOR UPDATE SKIP LOCKED`, so several replicas can run concurrently;
//! a lease allows a crashed worker's row to be reclaimed. Delivery is
//! deliberately at-least-once: `event_id` and `idempotency_key` make retries
//! safe for consumers, while Kafka outages remain invisible to HTTP writers.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::{
    database::DbPool,
    errors::{AppError, AppResult},
};

pub const DEFAULT_EVENT_VERSION: i32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainEventEnvelope<T> {
    pub event_id: Uuid,
    pub idempotency_key: String,
    pub aggregate_type: String,
    pub aggregate_id: Uuid,
    pub event_type: String,
    pub event_version: i32,
    pub occurred_at: DateTime<Utc>,
    pub payload: T,
}

impl<T> DomainEventEnvelope<T> {
    pub fn new(
        idempotency_key: impl Into<String>,
        aggregate_type: impl Into<String>,
        aggregate_id: Uuid,
        event_type: impl Into<String>,
        payload: T,
    ) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            idempotency_key: idempotency_key.into(),
            aggregate_type: aggregate_type.into(),
            aggregate_id,
            event_type: event_type.into(),
            event_version: DEFAULT_EVENT_VERSION,
            occurred_at: Utc::now(),
            payload,
        }
    }

    pub fn with_version(mut self, version: i32) -> Self {
        self.event_version = version.max(1);
        self
    }
}

#[derive(Debug, Clone)]
pub struct OutboxEvent {
    pub event_id: Uuid,
    pub idempotency_key: String,
    pub aggregate_type: String,
    pub aggregate_id: Uuid,
    pub event_type: String,
    pub event_version: i32,
    pub topic: String,
    pub payload: serde_json::Value,
    pub occurred_at: DateTime<Utc>,
}

impl OutboxEvent {
    pub fn from_envelope<T: Serialize>(
        envelope: DomainEventEnvelope<T>,
        topic: impl Into<String>,
    ) -> AppResult<Self> {
        if envelope.idempotency_key.trim().is_empty() {
            return Err(AppError::Validation {
                field: "idempotency_key".into(),
                message: "outbox idempotency key is required".into(),
            });
        }
        if envelope.event_type.trim().is_empty() {
            return Err(AppError::Validation {
                field: "event_type".into(),
                message: "outbox event type is required".into(),
            });
        }
        let topic = topic.into();
        if topic.trim().is_empty() {
            return Err(AppError::Validation {
                field: "topic".into(),
                message: "outbox topic is required".into(),
            });
        }
        Ok(Self {
            event_id: envelope.event_id,
            idempotency_key: envelope.idempotency_key,
            aggregate_type: envelope.aggregate_type,
            aggregate_id: envelope.aggregate_id,
            event_type: envelope.event_type,
            event_version: envelope.event_version,
            topic,
            payload: serde_json::to_value(envelope.payload).map_err(|error| {
                AppError::Internal(anyhow::Error::new(error).context("serialize outbox payload"))
            })?,
            occurred_at: envelope.occurred_at,
        })
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct OutboxRecord {
    pub id: Uuid,
    pub event_id: Uuid,
    pub idempotency_key: String,
    pub aggregate_type: String,
    pub aggregate_id: Uuid,
    pub event_type: String,
    pub event_version: i32,
    pub topic: String,
    pub payload: serde_json::Value,
    pub occurred_at: DateTime<Utc>,
    pub attempts: i32,
}

#[derive(Clone)]
pub struct OutboxRepository {
    db: DbPool,
}

impl OutboxRepository {
    pub fn new(db: DbPool) -> Self {
        Self { db }
    }

    /// Enqueue on an existing domain transaction. This is the reusable seam
    /// domain repositories call before committing their state mutation.
    pub async fn enqueue_on(
        &self,
        conn: &mut PgConnection,
        event: &OutboxEvent,
    ) -> AppResult<Uuid> {
        let id = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO domain_outbox (event_id, idempotency_key, aggregate_type, aggregate_id, event_type, event_version, topic, payload, occurred_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9) ON CONFLICT (idempotency_key) DO UPDATE SET idempotency_key = EXCLUDED.idempotency_key RETURNING id",
        )
        .bind(event.event_id).bind(&event.idempotency_key).bind(&event.aggregate_type)
        .bind(event.aggregate_id).bind(&event.event_type).bind(event.event_version)
        .bind(&event.topic).bind(&event.payload).bind(event.occurred_at)
        .fetch_one(conn).await?;
        Ok(id)
    }

    /// Convenience for infrastructure-only events. Domain writes should use
    /// [`Self::enqueue_on`] so the event and state share one transaction.
    pub async fn enqueue(&self, event: &OutboxEvent) -> AppResult<Uuid> {
        let mut tx: sqlx::Transaction<'_, sqlx::Postgres> = self.db.write().begin().await?;
        let id = self.enqueue_on(&mut tx, event).await?;
        tx.commit().await?;
        Ok(id)
    }

    /// Claim available work atomically. Stale processing leases are reclaimed
    /// after `lease_for`, which covers a worker crash without double-claiming
    /// live work across publisher replicas.
    pub async fn claim_batch(
        &self,
        worker_id: &str,
        limit: i64,
        lease_for: Duration,
    ) -> AppResult<Vec<OutboxRecord>> {
        let rows = sqlx::query_as::<_, OutboxRecord>(
            "WITH candidates AS (SELECT id FROM domain_outbox WHERE ((status IN ('pending','retry') AND available_at <= now()) OR (status = 'processing' AND locked_at < now() - $3::interval)) ORDER BY created_at, id FOR UPDATE SKIP LOCKED LIMIT $1) UPDATE domain_outbox o SET status = 'processing', attempts = o.attempts + 1, locked_at = now(), locked_by = $2, updated_at = now() FROM candidates c WHERE o.id = c.id RETURNING o.id, o.event_id, o.idempotency_key, o.aggregate_type, o.aggregate_id, o.event_type, o.event_version, o.topic, o.payload, o.occurred_at, o.attempts",
        )
        .bind(limit.clamp(1, 500)).bind(worker_id).bind(format!("{} seconds", lease_for.num_seconds().max(1)))
        .fetch_all(self.db.write()).await?;
        Ok(rows)
    }

    pub async fn mark_published(&self, id: Uuid, worker_id: &str) -> AppResult<bool> {
        let result = sqlx::query("UPDATE domain_outbox SET status='published', published_at=now(), locked_at=NULL, locked_by=NULL, updated_at=now() WHERE id=$1 AND status='processing' AND locked_by=$2")
            .bind(id).bind(worker_id).execute(self.db.write()).await?;
        Ok(result.rows_affected() == 1)
    }

    /// Retry with bounded attempts. Once the cap is reached, the row is
    /// retained as `dead` for operator replay/audit instead of being dropped.
    pub async fn mark_failed(
        &self,
        id: Uuid,
        worker_id: &str,
        error: &str,
        next_attempt_at: DateTime<Utc>,
        max_attempts: i32,
    ) -> AppResult<bool> {
        let result = sqlx::query("UPDATE domain_outbox SET status = CASE WHEN attempts >= $5 THEN 'dead' ELSE 'retry' END, available_at = CASE WHEN attempts >= $5 THEN available_at ELSE $4 END, last_error = left($3, 4000), dead_lettered_at = CASE WHEN attempts >= $5 THEN now() ELSE dead_lettered_at END, locked_at=NULL, locked_by=NULL, updated_at=now() WHERE id=$1 AND status='processing' AND locked_by=$2")
            .bind(id).bind(worker_id).bind(error).bind(next_attempt_at).bind(max_attempts.max(1)).execute(self.db.write()).await?;
        Ok(result.rows_affected() == 1)
    }
}

#[derive(Debug, Default)]
pub struct PublisherMetrics {
    pub claimed: std::sync::atomic::AtomicU64,
    pub published: std::sync::atomic::AtomicU64,
    pub retried: std::sync::atomic::AtomicU64,
    pub dead_lettered: std::sync::atomic::AtomicU64,
    pub kafka_failures: std::sync::atomic::AtomicU64,
}

impl PublisherMetrics {
    pub fn render_prometheus(&self) -> String {
        use std::sync::atomic::Ordering::Relaxed;
        format!("# HELP stocklink_outbox_events_claimed_total Claimed outbox rows.\n# TYPE stocklink_outbox_events_claimed_total counter\nstocklink_outbox_events_claimed_total {}\n# HELP stocklink_outbox_events_published_total Successfully published outbox rows.\n# TYPE stocklink_outbox_events_published_total counter\nstocklink_outbox_events_published_total {}\n# HELP stocklink_outbox_events_retried_total Rows returned to retry.\n# TYPE stocklink_outbox_events_retried_total counter\nstocklink_outbox_events_retried_total {}\n# HELP stocklink_outbox_events_dead_lettered_total Rows moved to dead-letter state.\n# TYPE stocklink_outbox_events_dead_lettered_total counter\nstocklink_outbox_events_dead_lettered_total {}\n# HELP stocklink_outbox_kafka_failures_total Kafka publish failures.\n# TYPE stocklink_outbox_kafka_failures_total counter\nstocklink_outbox_kafka_failures_total {}\n", self.claimed.load(Relaxed), self.published.load(Relaxed), self.retried.load(Relaxed), self.dead_lettered.load(Relaxed), self.kafka_failures.load(Relaxed))
    }
}
