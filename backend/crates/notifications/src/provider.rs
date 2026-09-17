//! Push notification provider abstraction. Mirrors `email::gateway` /
//! `sms::gateway` in the other services: [`NotificationProvider`] is the
//! boundary, [`FcmNotificationProvider`] (`super::fcm`) is the real
//! provider, wired up only when `FcmConfig` is present (see `state.rs`) —
//! there's no logging stub here because push is already fully optional at
//! the call site (`NotificationService::dispatch` only reaches for it when
//! `Some`), unlike email/OTP delivery which always needs *some* gateway.
//!
//! [`FcmNotificationProvider`]: super::fcm::FcmNotificationProvider

use async_trait::async_trait;

/// One registered device to push to.
#[derive(Debug, Clone)]
pub struct PushTarget {
    pub token: String,
}

/// The notification content, independent of any specific provider's wire
/// format. `Generic` is the only variant today — every dispatch goes
/// through `NotificationService::dispatch`'s free-text title/body; a
/// richer event (e.g. carrying deep-link data) would be a new variant, not
/// a change to this one.
#[derive(Debug, Clone)]
pub enum NotificationEvent {
    Generic { title: String, body: String },
}

/// Opaque delivery failure. Push is always a best-effort secondary to the
/// in-app notification (the record of truth) — see `service.rs` — so
/// callers log and continue rather than treat this as fatal.
#[derive(Debug, Clone, thiserror::Error)]
#[error("push delivery failed: {0}")]
pub struct PushError(pub String);

#[async_trait]
pub trait NotificationProvider: Send + Sync {
    async fn send(
        &self,
        event: &NotificationEvent,
        targets: &[PushTarget],
    ) -> Result<(), PushError>;
}
