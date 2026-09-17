//! Commerce service entry point.
//!
//! `stocklink-commerce migrate` applies pending migrations and exits.
//! Run with no arguments, the server refuses to start against a database
//! with migrations still pending, rather than applying them itself at boot
//! — auto-migrating is exactly wrong the moment there's more than one
//! replica, since two processes racing the same migration is how a rollout
//! corrupts a schema. A real deployment runs `migrate` once, then starts
//! every replica.

use stocklink_commerce::config::Config;
use stocklink_commerce::routes;
use stocklink_commerce::state::AppState;
use stocklink_shared::database::{self, redis, DbPool};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let config = Config::from_env().expect("invalid configuration — see the error above");
    let _tracing = stocklink_shared::utils::observability::init_tracing(&config.observability);

    let db = database::connect(&config.database)
        .await
        .expect("failed to connect to the database");

    if std::env::args().nth(1).as_deref() == Some("migrate") {
        MIGRATOR
            .run(db.write())
            .await
            .expect("failed to apply migrations");
        tracing::info!("migrations applied");
        return;
    }

    if migrations_are_pending(&db).await {
        eprintln!("refusing to start: pending migrations. Run `stocklink-commerce migrate` first.");
        std::process::exit(1);
    }

    let redis = redis::connect(&config.redis).expect("failed to build the redis pool");

    let host = config.server.host.clone();
    let port = config.server.port;
    let state = AppState::new(config, db, redis);
    let router = routes::build_router(state);

    let listener = tokio::net::TcpListener::bind((host.as_str(), port))
        .await
        .unwrap_or_else(|err| panic!("failed to bind {host}:{port}: {err}"));
    tracing::info!(%host, port, "stocklink-commerce listening");

    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .expect("server error");
}

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

async fn migrations_are_pending(db: &DbPool) -> bool {
    let applied: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations WHERE success = true")
            .fetch_one(db.write())
            .await
            .unwrap_or(0);
    (applied as usize) < MIGRATOR.migrations.len()
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install Ctrl+C handler");
    tracing::info!("shutdown signal received");
}
