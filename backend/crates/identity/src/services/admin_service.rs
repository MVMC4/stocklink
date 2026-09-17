//! Admin console: the audit log and the deep-link tiles (Grafana,
//! Prometheus, ...) shown on the console's landing page.

use uuid::Uuid;

use stocklink_shared::errors::AppResult;

use crate::admin_links::AdminLinksConfig;
use crate::repositories::admin_repository::{AdminAuditEntry, AdminRepository};

#[derive(Clone)]
pub struct AdminService {
    repo: AdminRepository,
    links: AdminLinksConfig,
}

impl AdminService {
    pub fn new(repo: AdminRepository, links: AdminLinksConfig) -> Self {
        Self { repo, links }
    }

    pub async fn audit_log(&self, limit: i64) -> AppResult<Vec<AdminAuditEntry>> {
        self.repo.recent(limit).await
    }

    pub fn links(&self) -> &AdminLinksConfig {
        &self.links
    }

    pub async fn record(
        &self,
        actor_account_id: Uuid,
        action: &str,
        target_type: Option<&str>,
        target_id: Option<Uuid>,
    ) -> AppResult<()> {
        self.repo
            .record(
                actor_account_id,
                action,
                target_type,
                target_id,
                serde_json::Value::Object(Default::default()),
            )
            .await
    }
}
