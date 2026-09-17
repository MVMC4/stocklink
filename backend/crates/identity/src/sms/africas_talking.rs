//! Africa's Talking bulk SMS gateway (used for phone-channel OTP delivery).
//! Their API takes form-encoded POST bodies and an `apiKey` header, and
//! reports per-recipient delivery status in the response body rather than
//! through the HTTP status code alone — a 2xx response can still contain a
//! rejected recipient.

use async_trait::async_trait;
use serde::Deserialize;

use stocklink_shared::config::AfricaTalkingConfig;

use super::gateway::{SmsError, SmsGateway};

pub struct AfricaTalkingSmsGateway {
    client: reqwest::Client,
    username: String,
    api_key: String,
    base_url: String,
    sender_id: Option<String>,
}

impl AfricaTalkingSmsGateway {
    pub fn new(config: &AfricaTalkingConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .expect("reqwest client with a fixed timeout always builds");
        Self {
            client,
            username: config.username.clone(),
            api_key: config.api_key.clone(),
            base_url: config.base_url.clone(),
            sender_id: config.sender_id.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct SendSmsResponse {
    #[serde(rename = "SMSMessageData")]
    data: SmsMessageData,
}

#[derive(Debug, Deserialize)]
struct SmsMessageData {
    #[serde(rename = "Recipients")]
    recipients: Vec<Recipient>,
}

#[derive(Debug, Deserialize)]
struct Recipient {
    status: String,
    #[serde(rename = "statusCode")]
    status_code: i32,
}

#[async_trait]
impl SmsGateway for AfricaTalkingSmsGateway {
    async fn send(&self, to: &str, message: &str) -> Result<(), SmsError> {
        let mut form = vec![
            ("username", self.username.as_str()),
            ("to", to),
            ("message", message),
        ];
        if let Some(sender_id) = &self.sender_id {
            form.push(("from", sender_id.as_str()));
        }

        let response = self
            .client
            .post(&self.base_url)
            .header("apiKey", &self.api_key)
            .header("Accept", "application/json")
            .form(&form)
            .send()
            .await
            .map_err(|err| SmsError(err.to_string()))?;

        if !response.status().is_success() {
            return Err(SmsError(format!(
                "africa's talking returned {}",
                response.status()
            )));
        }

        let body: SendSmsResponse = response
            .json()
            .await
            .map_err(|err| SmsError(format!("could not parse response: {err}")))?;

        // 100/101 are Africa's Talking's "success"/"queued" codes; anything
        // else is a per-recipient rejection even though the HTTP call itself
        // succeeded (invalid number, insufficient balance, blacklisted, ...).
        match body.data.recipients.first() {
            Some(recipient) if recipient.status_code == 100 || recipient.status_code == 101 => {
                Ok(())
            }
            Some(recipient) => Err(SmsError(format!(
                "recipient rejected: {} ({})",
                recipient.status, recipient.status_code
            ))),
            None => Err(SmsError("no recipient in response".to_string())),
        }
    }
}
