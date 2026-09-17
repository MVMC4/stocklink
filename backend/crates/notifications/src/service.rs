//! Creates the in-app notification row and, when push is configured,
//! dispatches it to the account's registered devices too. Push failures never
//! fail the caller — the in-app notification (the record of truth) is
//! already written.

use std::sync::Arc;

use uuid::Uuid;

use crate::models::notification::Notification;
use crate::provider::{NotificationEvent, NotificationProvider};
use crate::repositories::device_repository::DeviceRepository;
use crate::repositories::NotificationRepository;
use stocklink_shared::errors::AppResult;

#[derive(Clone)]
pub struct NotificationService {
    repo: NotificationRepository,
    devices: DeviceRepository,
    push: Option<Arc<dyn NotificationProvider>>,
}

impl NotificationService {
    pub fn new(
        repo: NotificationRepository,
        devices: DeviceRepository,
        push: Option<Arc<dyn NotificationProvider>>,
    ) -> Self {
        Self {
            repo,
            devices,
            push,
        }
    }

    /// Writes the in-app notification and, if push is configured and the
    /// account has registered devices, attempts a push too. Used both by
    /// the internal HTTP endpoint (any caller, generic title/body) and
    /// could grow typed convenience wrappers again if a richer push payload
    /// (e.g. deep-link data) is ever needed for a specific event.
    pub async fn dispatch(
        &self,
        account_id: Uuid,
        kind: &str,
        title: &str,
        body: &str,
        data: serde_json::Value,
    ) -> AppResult<()> {
        self.repo
            .create(account_id, kind, title, body, data)
            .await?;

        if let Some(push) = &self.push {
            let targets = self.devices.active_targets_for_account(account_id).await?;
            if !targets.is_empty() {
                let event = NotificationEvent::Generic {
                    title: title.to_string(),
                    body: body.to_string(),
                };
                if let Err(err) = push.send(&event, &targets).await {
                    tracing::warn!(error = ?err, "push dispatch failed; in-app notification still recorded");
                }
            }
        }
        Ok(())
    }

    pub async fn list_for_account(&self, account_id: Uuid) -> AppResult<Vec<Notification>> {
        self.repo.list_for_account(account_id, 50).await
    }

    pub async fn mark_read(&self, account_id: Uuid, id: Uuid) -> AppResult<bool> {
        self.repo.mark_read(account_id, id).await
    }
}
