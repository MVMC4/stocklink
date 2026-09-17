//! Background retention reaper: periodically deletes expired OTP challenges
//! and expired/revoked sessions so these high-churn tables don't grow
//! unbounded.

use crate::state::AppState;

pub struct CleanupScheduler;

impl CleanupScheduler {
    /// Spawns the periodic sweep as a detached background task. A no-op when
    /// `config.maintenance.enabled` is false.
    pub fn spawn(state: AppState) {
        if !state.config.maintenance.enabled {
            return;
        }
        let interval = state.config.maintenance.interval;
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            loop {
                ticker.tick().await;
                if let Err(err) = sweep(&state).await {
                    tracing::warn!(error = ?err, "maintenance sweep failed");
                }
            }
        });
    }
}

async fn sweep(state: &AppState) -> anyhow::Result<()> {
    let otp_deleted =
        sqlx::query("DELETE FROM otp_challenges WHERE expires_at < now() - interval '1 day'")
            .execute(state.db.write())
            .await?
            .rows_affected();

    let sessions_deleted = sqlx::query(
        "DELETE FROM sessions WHERE expires_at < now() - interval '1 day' OR revoked_at < now() - interval '1 day'",
    )
    .execute(state.db.write())
    .await?
    .rows_affected();

    if otp_deleted > 0 || sessions_deleted > 0 {
        tracing::info!(otp_deleted, sessions_deleted, "maintenance sweep complete");
    }
    Ok(())
}
