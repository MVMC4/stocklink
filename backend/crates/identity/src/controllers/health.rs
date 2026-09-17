//! Liveness, readiness and the Prometheus scrape endpoint. `/health/live`
//! answers "is the process up" with no dependency checks — a database
//! outage must not make the orchestrator think the process itself is dead
//! and restart it, which would only add load to an already-struggling
//! database. `/health/ready` is the one that actually pings dependencies.

use axum::extract::State;
use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use stocklink_shared::database::redis;

use crate::schemas::common::{DependencyStatus, ReadyRes};
use crate::state::AppState;

pub async fn live() -> StatusCode {
    StatusCode::OK
}

async fn check_dependencies(state: &AppState) -> Vec<DependencyStatus> {
    vec![
        DependencyStatus {
            name: "database",
            healthy: state.db.is_healthy().await,
        },
        DependencyStatus {
            name: "redis",
            healthy: redis::is_healthy(&state.redis).await,
        },
    ]
}

pub async fn ready(State(state): State<AppState>) -> impl IntoResponse {
    let dependencies = check_dependencies(&state).await;
    let all_healthy = dependencies.iter().all(|d| d.healthy);
    let status = if all_healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (status, Json(ReadyRes::from_dependencies(dependencies)))
}

pub async fn metrics(State(state): State<AppState>) -> impl IntoResponse {
    let dependencies = check_dependencies(&state).await;
    let db_up = dependencies
        .iter()
        .any(|d| d.name == "database" && d.healthy);
    let redis_up = dependencies.iter().any(|d| d.name == "redis" && d.healthy);

    let mut body = state.metrics.render_prometheus();
    body.push_str(&stocklink_shared::utils::metrics::Metrics::render_health(
        db_up,
        redis_up,
        env!("CARGO_PKG_VERSION"),
    ));

    ([(header::CONTENT_TYPE, "text/plain; version=0.0.4")], body)
}
