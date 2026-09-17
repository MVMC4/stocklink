//! Real Resend transactional-email transport.
//!
//! Wire contract verified against the official Resend documentation
//! (<https://resend.com/docs/api-reference/emails/send-email> and
//! `/docs/api-reference/errors>) on 9 September 2026:
//!
//! - `POST https://api.resend.com/emails`
//! - `Authorization: Bearer re_...`, `Content-Type: application/json`
//! - body: `from` (`"Name <addr>"`), `to` (string | string[]), `subject`
//!   required; `text`, `html`, `reply_to` optional.
//! - success: `200` with `{ "id": "<uuid>" }`.
//! - failure: `401`/`403` (auth / unverified sender domain / quota), `422`
//!   (validation), `429` (rate), `5xx`; error bodies carry a `message` field
//!   but the schema is not guaranteed, so this module only reports that
//!   delivery failed, never the raw provider response (rule 7).
//!
//! Mirrors `sms::africas_talking`: an inner `EmailTransport` trait so tests
//! substitute a fake instead of making network calls; a single attempt with a
//! short timeout (OTP email is best-effort — the code is already persisted).

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use tokio::time::timeout;

use crate::email::gateway::{mask_email, EmailError, EmailGateway, EmailMessage};
use stocklink_shared::config::EmailConfig;

#[derive(Debug, Deserialize)]
struct ResendSendResponse {
    id: String,
}

#[async_trait]
trait EmailTransport: Send + Sync {
    /// Returns the provider message id on success, or an opaque failure string.
    async fn send(&self, message: EmailMessage<'_>) -> Result<String, String>;
}

struct HttpEmailTransport {
    http: reqwest::Client,
    config: EmailConfig,
}

impl HttpEmailTransport {
    fn new(config: EmailConfig) -> Self {
        Self {
            http: reqwest::Client::new(),
            config,
        }
    }
}

#[async_trait]
impl EmailTransport for HttpEmailTransport {
    async fn send(&self, message: EmailMessage<'_>) -> Result<String, String> {
        let from = format!("{} <{}>", self.config.from_name, self.config.from_address);
        let mut body = serde_json::json!({
            "from": from,
            // Single-element array: Resend accepts a string or an array; the
            // array form is unambiguous.
            "to": [message.to],
            "subject": message.subject,
            "text": message.text_body,
        });
        if let Some(html) = message.html_body {
            body["html"] = serde_json::Value::String(html.to_string());
        }
        if let Some(reply_to) = self.config.reply_to.as_deref() {
            body["reply_to"] = serde_json::Value::String(reply_to.to_string());
        }

        let sent = timeout(
            self.config.timeout,
            self.http
                .post(format!("{}/emails", self.config.base_url))
                .bearer_auth(&self.config.api_key)
                .json(&body)
                .send(),
        )
        .await;

        let response = match sent {
            Err(_) => return Err("request timed out".to_string()),
            Ok(Err(err)) => return Err(err.to_string()),
            Ok(Ok(response)) => response,
        };

        let status = response.status();
        if !status.is_success() {
            // Never surface the raw provider body — it can echo the recipient
            // or subject. The status class is enough to act on.
            return Err(format!("provider returned {status}"));
        }

        let parsed: ResendSendResponse = response
            .json()
            .await
            .map_err(|err| format!("invalid provider response: {err}"))?;
        Ok(parsed.id)
    }
}

/// Real Resend email gateway. Single attempt, short timeout — OTP email is a
/// best-effort secondary channel, never the sole record of truth. Failure is
/// surfaced to the caller as `Err`; whether to swallow it is decided at the
/// call site (`AuthService::dispatch_email_otp`).
pub struct ResendEmailGateway {
    transport: Arc<dyn EmailTransport>,
}

impl ResendEmailGateway {
    pub fn new(config: &EmailConfig) -> Self {
        Self {
            transport: Arc::new(HttpEmailTransport::new(config.clone())),
        }
    }
}

#[async_trait]
impl EmailGateway for ResendEmailGateway {
    async fn send(&self, message: EmailMessage<'_>) -> Result<(), EmailError> {
        let id = self.transport.send(message).await.map_err(EmailError)?;
        tracing::info!(
            to = %mask_email(message.to),
            provider_message_id = %id,
            "resend email sent"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        EmailGateway, EmailMessage, EmailTransport, ResendEmailGateway, ResendSendResponse,
    };
    use async_trait::async_trait;
    use std::sync::Arc;

    struct FakeTransport {
        result: Result<String, String>,
    }

    #[async_trait]
    impl EmailTransport for FakeTransport {
        async fn send(&self, _message: EmailMessage<'_>) -> Result<String, String> {
            self.result.clone()
        }
    }

    fn gateway_with(result: Result<String, String>) -> ResendEmailGateway {
        ResendEmailGateway {
            transport: Arc::new(FakeTransport { result }),
        }
    }

    fn message() -> EmailMessage<'static> {
        EmailMessage {
            to: "buyer@acme.co.bw",
            subject: "Your code",
            text_body: "123456",
            html_body: Some("<p>123456</p>"),
        }
    }

    #[tokio::test]
    async fn send_succeeds_when_transport_returns_an_id() {
        assert!(gateway_with(Ok("msg-id".into()))
            .send(message())
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn send_surfaces_transport_failure_to_the_caller() {
        let result = gateway_with(Err("provider returned 422 Unprocessable Entity".into()))
            .send(message())
            .await;
        assert!(
            result.is_err(),
            "gateway-level failures are surfaced; swallowing is a caller decision"
        );
        // The error must not carry the recipient or subject.
        let rendered = result.unwrap_err().to_string();
        assert!(!rendered.contains("acme.co.bw") && !rendered.contains("Your code"));
    }

    #[test]
    fn parses_provider_message_id_from_success_body() {
        let parsed: ResendSendResponse =
            serde_json::from_str(r#"{"id":"49a3999c-0ce1-4ea6-ab68-afcd6dc2e794"}"#)
                .expect("valid resend response");
        assert_eq!(parsed.id, "49a3999c-0ce1-4ea6-ab68-afcd6dc2e794");
    }
}
