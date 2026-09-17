//! Admin console: audit log of privileged actions.

use uuid::Uuid;

use stocklink_shared::database::DbPool;
use stocklink_shared::errors::AppResult;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AdminAuditEntry {
    pub id: Uuid,
    pub actor_account_id: Uuid,
    pub action: String,
    pub target_type: Option<String>,
    pub target_id: Option<Uuid>,
    pub metadata: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone)]
pub struct AdminRepository {
    db: DbPool,
}

impl AdminRepository {
    pub fn new(db: DbPool) -> Self {
        Self { db }
    }

    pub async fn record(
        &self,
        actor_account_id: Uuid,
        action: &str,
        target_type: Option<&str>,
        target_id: Option<Uuid>,
        metadata: serde_json::Value,
    ) -> AppResult<()> {
        sqlx::query(
            "INSERT INTO admin_audit_log (actor_account_id, action, target_type, target_id, metadata) \
             VALUES ($1,$2,$3,$4,$5)",
        )
        .bind(actor_account_id)
        .bind(action)
        .bind(target_type)
        .bind(target_id)
        .bind(metadata)
        .execute(self.db.write())
        .await?;
        Ok(())
    }

    pub async fn recent(&self, limit: i64) -> AppResult<Vec<AdminAuditEntry>> {
        let rows = sqlx::query_as::<_, AdminAuditEntry>(
            "SELECT * FROM admin_audit_log ORDER BY created_at DESC LIMIT $1",
        )
        .bind(limit.clamp(1, 500))
        .fetch_all(self.db.read())
        .await?;
        Ok(rows)
    }
}
