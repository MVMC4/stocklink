use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::models::notification::Notification;

#[derive(Debug, Serialize, ToSchema)]
pub struct NotificationRes {
    pub id: Uuid,
    pub kind: String,
    pub title: String,
    pub body: String,
    pub data: serde_json::Value,
    pub read_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl From<Notification> for NotificationRes {
    fn from(n: Notification) -> Self {
        Self {
            id: n.id,
            kind: n.kind,
            title: n.title,
            body: n.body,
            data: n.data,
            read_at: n.read_at,
            created_at: n.created_at,
        }
    }
}
