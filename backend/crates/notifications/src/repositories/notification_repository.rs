//! In-app notification inbox (bell icon, `/v1/notifications`).

use uuid::Uuid;

use crate::models::notification::Notification;
use stocklink_shared::database::DbPool;
use stocklink_shared::errors::AppResult;

#[derive(Clone)]
pub struct NotificationRepository {
    db: DbPool,
}

impl NotificationRepository {
    pub fn new(db: DbPool) -> Self {
        Self { db }
    }

    pub async fn create(
        &self,
        account_id: Uuid,
        kind: &str,
        title: &str,
        body: &str,
        data: serde_json::Value,
    ) -> AppResult<Notification> {
        let row = sqlx::query_as::<_, Notification>(
            "INSERT INTO notifications (account_id, kind, title, body, data) VALUES ($1,$2,$3,$4,$5) RETURNING *",
        )
        .bind(account_id)
        .bind(kind)
        .bind(title)
        .bind(body)
        .bind(data)
        .fetch_one(self.db.write())
        .await?;
        Ok(row)
    }

    pub async fn list_for_account(
        &self,
        account_id: Uuid,
        limit: i64,
    ) -> AppResult<Vec<Notification>> {
        let rows = sqlx::query_as::<_, Notification>(
            "SELECT * FROM notifications WHERE account_id = $1 ORDER BY created_at DESC LIMIT $2",
        )
        .bind(account_id)
        .bind(limit.clamp(1, 200))
        .fetch_all(self.db.read())
        .await?;
        Ok(rows)
    }

    pub async fn mark_read(&self, account_id: Uuid, id: Uuid) -> AppResult<bool> {
        let result = sqlx::query(
            "UPDATE notifications SET read_at = now() WHERE id = $1 AND account_id = $2 AND read_at IS NULL",
        )
        .bind(id)
        .bind(account_id)
        .execute(self.db.write())
        .await?;
        Ok(result.rows_affected() == 1)
    }
}
