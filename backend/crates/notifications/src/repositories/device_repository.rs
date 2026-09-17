//! Registered push devices (`devices` table): registration and the active
//! target list `NotificationProvider` implementations send to.

use uuid::Uuid;

use stocklink_shared::database::DbPool;
use stocklink_shared::errors::AppResult;

use crate::provider::PushTarget;

#[derive(Clone)]
pub struct DeviceRepository {
    db: DbPool,
}

impl DeviceRepository {
    pub fn new(db: DbPool) -> Self {
        Self { db }
    }

    /// Re-registering an existing `(provider, token)` reassigns it to the
    /// presenting account and reactivates it — the common case in practice
    /// is the same device signing in as a different account (a shared
    /// tablet, a reinstall), not two accounts genuinely sharing one token.
    pub async fn register(&self, account_id: Uuid, provider: &str, token: &str) -> AppResult<()> {
        sqlx::query(
            "INSERT INTO devices (account_id, provider, token) VALUES ($1, $2, $3) \
             ON CONFLICT (provider, token) \
             DO UPDATE SET account_id = EXCLUDED.account_id, active = true",
        )
        .bind(account_id)
        .bind(provider)
        .bind(token)
        .execute(self.db.write())
        .await?;
        Ok(())
    }

    pub async fn active_targets_for_account(&self, account_id: Uuid) -> AppResult<Vec<PushTarget>> {
        let tokens: Vec<(String,)> =
            sqlx::query_as("SELECT token FROM devices WHERE account_id = $1 AND active = true")
                .bind(account_id)
                .fetch_all(self.db.read())
                .await?;
        Ok(tokens
            .into_iter()
            .map(|(token,)| PushTarget { token })
            .collect())
    }

    /// Called when a provider reports a token as no longer valid (FCM's
    /// `UNREGISTERED` error) — the device stays on record for history but
    /// stops receiving pushes until it registers again.
    pub async fn deactivate(&self, token: &str) -> AppResult<()> {
        sqlx::query("UPDATE devices SET active = false WHERE token = $1")
            .bind(token)
            .execute(self.db.write())
            .await?;
        Ok(())
    }
}
