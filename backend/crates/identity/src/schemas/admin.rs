use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, ToSchema)]
pub struct AdminAuditEntryRes {
    pub id: Uuid,
    pub actor_account_id: Uuid,
    pub action: String,
    pub target_type: Option<String>,
    pub target_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

impl From<crate::repositories::admin_repository::AdminAuditEntry> for AdminAuditEntryRes {
    fn from(e: crate::repositories::admin_repository::AdminAuditEntry) -> Self {
        Self {
            id: e.id,
            actor_account_id: e.actor_account_id,
            action: e.action,
            target_type: e.target_type,
            target_id: e.target_id,
            created_at: e.created_at,
        }
    }
}

#[derive(Debug, Serialize, ToSchema, Default)]
pub struct AdminLinksRes {
    pub grafana_url: Option<String>,
    pub prometheus_url: Option<String>,
    pub jira_url: Option<String>,
    pub status_url: Option<String>,
    pub docs_url: Option<String>,
}

impl From<&crate::admin_links::AdminLinksConfig> for AdminLinksRes {
    fn from(c: &crate::admin_links::AdminLinksConfig) -> Self {
        Self {
            grafana_url: c.grafana_url.clone(),
            prometheus_url: c.prometheus_url.clone(),
            jira_url: c.jira_url.clone(),
            status_url: c.status_url.clone(),
            docs_url: c.docs_url.clone(),
        }
    }
}
