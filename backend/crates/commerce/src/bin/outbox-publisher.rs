//! Durable Kafka publisher for the Postgres transactional outbox.
//!
//! This process is intentionally independent of the HTTP listener. A Kafka
//! outage only increases retry/dead-letter state; it cannot make a domain
//! write wait for Kafka.

use std::{env, time::Duration as StdDuration};

use anyhow::{Context, Result};
use axum::{
    extract::State,
    http::{header, HeaderValue},
    response::IntoResponse,
    routing::get,
    Router,
};
use chrono::{Duration, Utc};
use rdkafka::{
    producer::{FutureProducer, FutureRecord},
    ClientConfig,
};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use stocklink_shared::{
    database::DbPool,
    outbox::{OutboxRepository, PublisherMetrics},
};
use tokio::time::{interval, MissedTickBehavior};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("stocklink_outbox=info".parse()?),
        )
        .json()
        .init();

    let database_url = env::var("DATABASE_URL").context("DATABASE_URL is required")?;
    let kafka_brokers = env::var("KAFKA_BROKERS").context("KAFKA_BROKERS is required")?;
    let pool = PgPoolOptions::new()
        .max_connections(env_u32("OUTBOX_DB_MAX_CONNECTIONS", 5))
        .connect(&database_url)
        .await
        .context("connect Postgres")?;
    let db = DbPool::from(pool);
    let repo = OutboxRepository::new(db);
    let mut kafka = ClientConfig::new();
    kafka
        .set("bootstrap.servers", &kafka_brokers)
        .set("enable.idempotence", "true")
        .set("acks", "all")
        .set(
            "message.timeout.ms",
            &env_u64("OUTBOX_KAFKA_MESSAGE_TIMEOUT_MS", 10_000).to_string(),
        )
        .set("compression.type", "snappy");
    for (key, env_key) in [
        ("security.protocol", "KAFKA_SECURITY_PROTOCOL"),
        ("ssl.ca.location", "KAFKA_CA_LOCATION"),
        ("ssl.certificate.location", "KAFKA_CERTIFICATE_LOCATION"),
        ("ssl.key.location", "KAFKA_KEY_LOCATION"),
    ] {
        if let Ok(value) = env::var(env_key) {
            kafka.set(key, &value);
        }
    }
    let producer: FutureProducer = kafka.create().context("create idempotent Kafka producer")?;

    let worker_id = env::var("OUTBOX_WORKER_ID")
        .unwrap_or_else(|_| format!("publisher-{}", uuid::Uuid::new_v4()));
    let batch_size = env_i64("OUTBOX_BATCH_SIZE", 100).clamp(1, 500);
    let poll = StdDuration::from_secs(env_u64("OUTBOX_POLL_INTERVAL_SECS", 2).max(1));
    let lease = Duration::seconds(env_u64("OUTBOX_LEASE_SECS", 120).max(10) as i64);
    let max_attempts = env_i32("OUTBOX_MAX_ATTEMPTS", 12).max(1);
    let metrics = Arc::new(PublisherMetrics::default());
    let metrics_addr =
        env::var("OUTBOX_METRICS_ADDR").unwrap_or_else(|_| "0.0.0.0:9091".to_string());
    let metrics_listener = tokio::net::TcpListener::bind(&metrics_addr)
        .await
        .with_context(|| format!("bind OUTBOX_METRICS_ADDR {metrics_addr}"))?;
    let metrics_app = Router::new()
        .route("/metrics", get(metrics_handler))
        .with_state(metrics.clone());
    tokio::spawn(async move {
        if let Err(error) = axum::serve(metrics_listener, metrics_app).await {
            tracing::error!(?error, "outbox metrics server stopped");
        }
    });
    let mut ticker = interval(poll);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    let mut report_tick = interval(StdDuration::from_secs(30));
    let shutdown = shutdown_signal();
    tokio::pin!(shutdown);

    tracing::info!(worker = %worker_id, brokers = %kafka_brokers, batch_size, max_attempts, "transactional outbox publisher started");
    loop {
        tokio::select! {
            _ = ticker.tick() => publish_batch(&repo, &producer, &worker_id, batch_size, lease, max_attempts, &metrics).await,
            _ = report_tick.tick() => tracing::info!(metrics = %metrics.render_prometheus(), "outbox publisher metrics"),
            _ = &mut shutdown => {
                tracing::info!(worker = %worker_id, "outbox publisher stopping");
                break;
            }
        }
    }
    Ok(())
}

async fn metrics_handler(State(metrics): State<Arc<PublisherMetrics>>) -> impl IntoResponse {
    let mut response = metrics.render_prometheus().into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/plain; version=0.0.4"),
    );
    response
}

async fn publish_batch(
    repo: &OutboxRepository,
    producer: &FutureProducer,
    worker_id: &str,
    batch_size: i64,
    lease: Duration,
    max_attempts: i32,
    metrics: &Arc<PublisherMetrics>,
) {
    let records = match repo.claim_batch(worker_id, batch_size, lease).await {
        Ok(records) => records,
        Err(error) => {
            tracing::error!(?error, "claiming outbox rows failed");
            return;
        }
    };
    if records.is_empty() {
        return;
    }
    use std::sync::atomic::Ordering::Relaxed;
    metrics.claimed.fetch_add(records.len() as u64, Relaxed);

    for record in records {
        let event = serde_json::json!({
            "event_id": record.event_id,
            "idempotency_key": record.idempotency_key,
            "aggregate_type": record.aggregate_type,
            "aggregate_id": record.aggregate_id,
            "event_type": record.event_type,
            "event_version": record.event_version,
            "occurred_at": record.occurred_at,
            "payload": record.payload,
        });
        let body = match serde_json::to_vec(&event) {
            Ok(body) => body,
            Err(error) => {
                tracing::error!(row = %record.id, ?error, "serializing outbox envelope failed");
                continue;
            }
        };
        let event_key = record.event_id.to_string();
        let result = producer
            .send(
                FutureRecord::to(&record.topic)
                    .key(&event_key)
                    .payload(&body),
                rdkafka::util::Timeout::After(StdDuration::from_secs(10)),
            )
            .await;
        match result {
            Ok(_delivery) => {
                if repo
                    .mark_published(record.id, worker_id)
                    .await
                    .unwrap_or(false)
                {
                    metrics.published.fetch_add(1, Relaxed);
                    tracing::debug!(row = %record.id, topic = %record.topic, "outbox event published");
                }
            }
            // rdkafka's FutureProducer returns the unsent message with the error.
            Err((error, _unsent)) => {
                metrics.kafka_failures.fetch_add(1, Relaxed);
                let delay_seconds = (2_i64
                    .saturating_pow(record.attempts.saturating_sub(1).min(12) as u32))
                .clamp(5, 900);
                let next = Utc::now() + Duration::seconds(delay_seconds);
                match repo
                    .mark_failed(record.id, worker_id, &error.to_string(), next, max_attempts)
                    .await
                {
                    Ok(true) if record.attempts >= max_attempts => {
                        metrics.dead_lettered.fetch_add(1, Relaxed);
                        tracing::error!(row = %record.id, attempts = record.attempts, "outbox event dead-lettered after Kafka failure");
                    }
                    Ok(true) => {
                        metrics.retried.fetch_add(1, Relaxed);
                        tracing::warn!(row = %record.id, retry_in_seconds = delay_seconds, "outbox event scheduled for retry");
                    }
                    Ok(false) => {
                        tracing::warn!(row = %record.id, "outbox failure could not update row (lease lost)")
                    }
                    Err(update_error) => {
                        tracing::error!(row = %record.id, ?update_error, "recording outbox failure failed; lease will expire for recovery")
                    }
                }
            }
        }
    }
}

fn env_u64(key: &str, default: u64) -> u64 {
    env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}
fn env_u32(key: &str, default: u32) -> u32 {
    env_u64(key, default as u64) as u32
}
fn env_i64(key: &str, default: i64) -> i64 {
    env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}
fn env_i32(key: &str, default: i32) -> i32 {
    env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("install Ctrl-C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
