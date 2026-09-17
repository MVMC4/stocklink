//! SMS gateway abstraction and a non-production stub.
//!
//! Mirrors `email::gateway`: [`SmsGateway`] is the boundary,
//! [`AfricaTalkingSmsGateway`] (`super::africas_talking`) is the real
//! provider, and [`NoopSmsGateway`] is the non-production fake used by
//! local dev and tests when `AFRICAS_TALKING_API_KEY` isn't configured.
//!
//! [`AfricaTalkingSmsGateway`]: super::AfricaTalkingSmsGateway

use async_trait::async_trait;

/// Opaque delivery failure (network error, non-2xx from the provider,
/// rejected recipient). Phone OTP is a best-effort secondary channel (see
/// `auth_service::request_otp`), so callers log and continue rather than
/// abort on this.
#[derive(Debug, Clone, thiserror::Error)]
#[error("sms delivery failed: {0}")]
pub struct SmsError(pub String);

#[async_trait]
pub trait SmsGateway: Send + Sync {
    async fn send(&self, to: &str, message: &str) -> Result<(), SmsError>;
}

/// Non-production gateway: no network call, always succeeds, logs only
/// non-sensitive metadata. Used whenever `AfricaTalkingConfig` isn't
/// present (see `state.rs`) — never in a production-like environment,
/// where an unconfigured provider means phone OTP is simply unavailable
/// (`AppError::ServiceUnavailable`), not silently faked.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoopSmsGateway;

#[async_trait]
impl SmsGateway for NoopSmsGateway {
    async fn send(&self, to: &str, message: &str) -> Result<(), SmsError> {
        tracing::info!(
            to = %mask_phone(to),
            message_len = message.len(),
            "stub sms gateway accepted message"
        );
        Ok(())
    }
}

/// Mask a phone number for logging: country code plus last two digits
/// (`+267***34`). Never emit the full number (rule 7).
pub(crate) fn mask_phone(phone: &str) -> String {
    if phone.len() <= 4 {
        return "***".to_string();
    }
    let prefix: String = phone.chars().take(4).collect();
    let suffix: String = phone
        .chars()
        .rev()
        .take(2)
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    format!("{prefix}***{suffix}")
}

#[cfg(test)]
mod tests {
    use super::{mask_phone, NoopSmsGateway, SmsGateway};

    #[tokio::test]
    async fn noop_gateway_accepts_every_message() {
        let result = NoopSmsGateway
            .send("+26771234567", "your code is 123456")
            .await;
        assert!(result.is_ok());
    }

    #[test]
    fn masks_middle_digits_but_keeps_prefix_and_last_two() {
        assert_eq!(mask_phone("+26771234567"), "+267***67");
        assert_eq!(mask_phone("12"), "***");
    }
}
