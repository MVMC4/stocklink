//! Firebase Cloud Messaging (HTTP v1 API) push provider. Authenticates as
//! the configured service account via the OAuth2 JWT-bearer grant (RFC
//! 7523): sign a short-lived JWT asserting the service account's identity
//! and the `firebase.messaging` scope, exchange it at Google's token
//! endpoint for a bearer access token, then call FCM with that token.
//!
//! Fetches a fresh access token per `send` call rather than caching one —
//! push notifications aren't a hot path (unlike, say, per-request auth), so
//! the extra round trip isn't worth the complexity of a shared,
//! expiry-aware token cache.

use async_trait::async_trait;
use chrono::Utc;
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};

use stocklink_shared::config::FcmConfig;

use crate::provider::{NotificationEvent, NotificationProvider, PushError, PushTarget};
use crate::repositories::device_repository::DeviceRepository;

const MESSAGING_SCOPE: &str = "https://www.googleapis.com/auth/firebase.messaging";

pub struct FcmNotificationProvider {
    client: reqwest::Client,
    project_id: String,
    client_email: String,
    encoding_key: EncodingKey,
    token_uri: String,
    devices: DeviceRepository,
}

impl FcmNotificationProvider {
    pub fn new(config: &FcmConfig, devices: DeviceRepository) -> Self {
        let client = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .expect("reqwest client with a fixed timeout always builds");
        let encoding_key = EncodingKey::from_rsa_pem(config.private_key.as_bytes())
            .expect("FCM_PRIVATE_KEY must be a valid RSA private key in PEM format");
        Self {
            client,
            project_id: config.project_id.clone(),
            client_email: config.client_email.clone(),
            encoding_key,
            token_uri: config.token_uri.clone(),
            devices,
        }
    }

    async fn fetch_access_token(&self) -> Result<String, PushError> {
        let now = Utc::now().timestamp();
        let claims = OAuthClaims {
            iss: self.client_email.clone(),
            scope: MESSAGING_SCOPE.to_string(),
            aud: self.token_uri.clone(),
            iat: now,
            exp: now + 3600,
        };
        let assertion =
            jsonwebtoken::encode(&Header::new(Algorithm::RS256), &claims, &self.encoding_key)
                .map_err(|err| PushError(format!("failed to sign FCM assertion: {err}")))?;

        let response = self
            .client
            .post(&self.token_uri)
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
                ("assertion", &assertion),
            ])
            .send()
            .await
            .map_err(|err| PushError(format!("token request failed: {err}")))?;

        if !response.status().is_success() {
            return Err(PushError(format!(
                "token endpoint returned {}",
                response.status()
            )));
        }

        let body: TokenResponse = response
            .json()
            .await
            .map_err(|err| PushError(format!("could not parse token response: {err}")))?;
        Ok(body.access_token)
    }

    async fn send_one(&self, access_token: &str, target: &str, title: &str, body: &str) {
        let url = format!(
            "https://fcm.googleapis.com/v1/projects/{}/messages:send",
            self.project_id
        );
        let payload = SendRequest {
            message: Message {
                token: target.to_string(),
                notification: NotificationPayload {
                    title: title.to_string(),
                    body: body.to_string(),
                },
            },
        };

        let response = match self
            .client
            .post(&url)
            .bearer_auth(access_token)
            .json(&payload)
            .send()
            .await
        {
            Ok(response) => response,
            Err(err) => {
                tracing::warn!(error = ?err, "fcm send request failed");
                return;
            }
        };

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            // FCM's HTTP v1 API reports an unregistered/invalid token as a
            // 404 on the messages:send call itself.
            if let Err(err) = self.devices.deactivate(target).await {
                tracing::warn!(error = ?err, "failed to deactivate unregistered fcm token");
            }
            return;
        }
        if !response.status().is_success() {
            tracing::warn!(status = %response.status(), "fcm send rejected");
        }
    }
}

#[async_trait]
impl NotificationProvider for FcmNotificationProvider {
    async fn send(
        &self,
        event: &NotificationEvent,
        targets: &[PushTarget],
    ) -> Result<(), PushError> {
        if targets.is_empty() {
            return Ok(());
        }
        let access_token = self.fetch_access_token().await?;
        let NotificationEvent::Generic { title, body } = event;
        for target in targets {
            self.send_one(&access_token, &target.token, title, body)
                .await;
        }
        Ok(())
    }
}

#[derive(Serialize)]
struct OAuthClaims {
    iss: String,
    scope: String,
    aud: String,
    iat: i64,
    exp: i64,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

#[derive(Serialize)]
struct SendRequest {
    message: Message,
}

#[derive(Serialize)]
struct Message {
    token: String,
    notification: NotificationPayload,
}

#[derive(Serialize)]
struct NotificationPayload {
    title: String,
    body: String,
}
