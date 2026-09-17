//! Email gateway abstraction and a deterministic non-production stub.
//!
//! Mirrors `sms::gateway`: [`EmailGateway`] is the boundary, [`ResendEmailGateway`]
//! (`super::resend`) is the real provider, and [`LoggingEmailGateway`] is the
//! non-production fake used by local dev and tests.
//!
//! [`ResendEmailGateway`]: super::resend::ResendEmailGateway

use async_trait::async_trait;

/// Opaque delivery failure (network error, non-2xx from the provider, rejected
/// recipient). Callers decide whether a failed send aborts the surrounding
/// operation or is logged and swallowed — see each call site.
#[derive(Debug, Clone, thiserror::Error)]
#[error("email delivery failed: {0}")]
pub struct EmailError(pub String);

/// One transactional email. Borrows its fields — nothing is retained past the
/// `send` call.
#[derive(Debug, Clone, Copy)]
pub struct EmailMessage<'a> {
    pub to: &'a str,
    pub subject: &'a str,
    pub text_body: &'a str,
    pub html_body: Option<&'a str>,
}

#[async_trait]
pub trait EmailGateway: Send + Sync {
    async fn send(&self, message: EmailMessage<'_>) -> Result<(), EmailError>;
}

/// Deterministic non-production gateway: no network call, always succeeds,
/// logs only non-sensitive metadata.
///
/// It must never log the subject or body — an OTP email's body *is* the
/// secret. In dev/test the code still reaches the caller through
/// `OtpRequestAck.dev_code` (config-gated, impossible in staging/production),
/// so this stub does not hide a delivery the tester needs.
#[derive(Debug, Clone, Copy, Default)]
pub struct LoggingEmailGateway;

#[async_trait]
impl EmailGateway for LoggingEmailGateway {
    async fn send(&self, message: EmailMessage<'_>) -> Result<(), EmailError> {
        tracing::info!(
            to = %mask_email(message.to),
            subject_len = message.subject.len(),
            text_len = message.text_body.len(),
            has_html = message.html_body.is_some(),
            "stub email gateway accepted message"
        );
        Ok(())
    }
}

/// Mask an email for logging: first char of the local-part, then the domain
/// (`b***@acme.co.bw`). Never emit the full recipient (rule 7).
pub(crate) fn mask_email(email: &str) -> String {
    match email.split_once('@') {
        Some((local, domain)) => {
            let first = local.chars().next().unwrap_or('*');
            format!("{first}***@{domain}")
        }
        None => "***".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{mask_email, EmailGateway, EmailMessage, LoggingEmailGateway};

    #[tokio::test]
    async fn logging_gateway_accepts_every_message() {
        let result = LoggingEmailGateway
            .send(EmailMessage {
                to: "buyer@acme.co.bw",
                subject: "code",
                text_body: "123456",
                html_body: None,
            })
            .await;
        assert!(result.is_ok());
    }

    #[test]
    fn masks_local_part_but_keeps_domain() {
        assert_eq!(mask_email("buyer@acme.co.bw"), "b***@acme.co.bw");
        assert_eq!(mask_email("x@y.z"), "x***@y.z");
        assert_eq!(mask_email("not-an-email"), "***");
    }
}
