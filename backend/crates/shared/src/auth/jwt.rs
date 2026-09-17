//! Access-token signing and verification. One shared HS256 secret across
//! every service (`JWT_ACCESS_SECRET`) — a service verifies a token issued
//! by identity without a network call, which is the whole point of the
//! split (see `docs/STATUS.md`'s microservices entry). Refresh tokens are a
//! separate, opaque, database-backed session (`identity::otp`) — never a
//! JWT, so there's nothing to verify offline for them and revocation is a
//! plain row update.

use chrono::Utc;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use uuid::Uuid;

use crate::auth::claims::TokenClaims;
use crate::config::JwtConfig;

pub struct JwtKeys {
    encoding: EncodingKey,
    decoding: DecodingKey,
    issuer: String,
    audience: String,
    access_ttl_secs: i64,
}

impl JwtKeys {
    pub fn new(config: &JwtConfig) -> Self {
        Self {
            encoding: EncodingKey::from_secret(config.access_secret.as_bytes()),
            decoding: DecodingKey::from_secret(config.access_secret.as_bytes()),
            issuer: config.issuer.clone(),
            audience: config.audience.clone(),
            access_ttl_secs: config.access_ttl.as_secs() as i64,
        }
    }

    pub fn issue_access(
        &self,
        account_id: Uuid,
        roles: Vec<String>,
    ) -> Result<String, jsonwebtoken::errors::Error> {
        let now = Utc::now().timestamp();
        let claims = TokenClaims {
            sub: account_id,
            roles,
            jti: Uuid::new_v4(),
            iat: now,
            exp: now + self.access_ttl_secs,
            iss: self.issuer.clone(),
            aud: self.audience.clone(),
        };
        encode(&Header::new(Algorithm::HS256), &claims, &self.encoding)
    }

    pub fn verify(&self, token: &str) -> Result<TokenClaims, jsonwebtoken::errors::Error> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&[self.issuer.as_str()]);
        validation.set_audience(&[self.audience.as_str()]);
        Ok(decode::<TokenClaims>(token, &self.decoding, &validation)?.claims)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    fn keys() -> JwtKeys {
        JwtKeys::new(&JwtConfig {
            access_secret: "test-secret-at-least-32-bytes-long!!".to_string(),
            issuer: "stocklink-test".to_string(),
            audience: "stocklink-test".to_string(),
            access_ttl: Duration::from_secs(900),
            refresh_ttl: Duration::from_secs(60 * 60 * 24 * 30),
        })
    }

    #[test]
    fn issued_token_verifies_with_the_same_keys() {
        let keys = keys();
        let account_id = Uuid::new_v4();
        let token = keys
            .issue_access(account_id, vec!["warehouse".to_string()])
            .unwrap();

        let claims = keys.verify(&token).unwrap();
        assert_eq!(claims.sub, account_id);
        assert!(claims.has_role("warehouse"));
    }

    /// Red test: a token signed with a different secret must not verify —
    /// the whole point of a shared HS256 secret is that only holders of it
    /// can mint a token another service will accept.
    #[test]
    fn token_signed_with_a_different_secret_is_rejected() {
        let issuer = keys();
        let verifier = JwtKeys::new(&JwtConfig {
            access_secret: "a-completely-different-secret-value!".to_string(),
            issuer: "stocklink-test".to_string(),
            audience: "stocklink-test".to_string(),
            access_ttl: Duration::from_secs(900),
            refresh_ttl: Duration::from_secs(60 * 60 * 24 * 30),
        });

        let token = issuer
            .issue_access(Uuid::new_v4(), vec![])
            .expect("issue with original secret");
        assert!(verifier.verify(&token).is_err());
    }
}
