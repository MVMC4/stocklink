//! [`AuthUser`]: the axum extractor every authenticated handler takes
//! instead of reading headers itself. Accepts either an `Authorization:
//! Bearer` header (API/mobile clients) or the `sl_session` HttpOnly cookie
//! (the web app — WO-05: no tokens in `localStorage`), so both documented
//! auth styles resolve to the same extractor rather than each handler
//! picking one.

use async_trait::async_trait;
use axum::extract::FromRequestParts;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use axum_extra::extract::cookie::CookieJar;
use uuid::Uuid;

use crate::auth::claims::{TokenClaims, ROLE_WAREHOUSE};
use crate::auth::denylist::AccessTokenDenylist;
use crate::auth::jwt::JwtKeys;
use crate::errors::AppError;

pub const SESSION_COOKIE: &str = "sl_session";

/// Implemented by every service's `AppState` so [`AuthUser`] can verify a
/// token without knowing anything else about that service's state — mirrors
/// `middleware::InternalAuthState`'s reason for existing.
pub trait AuthState: Send + Sync {
    fn jwt(&self) -> &JwtKeys;
    fn denylist(&self) -> &AccessTokenDenylist;
}

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub account_id: Uuid,
    pub roles: Vec<String>,
    pub claims: TokenClaims,
}

impl AuthUser {
    pub fn has_role(&self, role: &str) -> bool {
        self.claims.has_role(role)
    }
}

fn bearer_token(parts: &Parts) -> Option<String> {
    parts
        .headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(str::to_string)
}

fn cookie_token(parts: &Parts) -> Option<String> {
    CookieJar::from_headers(&parts.headers)
        .get(SESSION_COOKIE)
        .map(|c| c.value().to_string())
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: AuthState + Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let token = bearer_token(parts)
            .or_else(|| cookie_token(parts))
            .ok_or(AppError::Unauthorized)?;

        let claims = state
            .jwt()
            .verify(&token)
            .map_err(|_| AppError::Unauthorized)?;

        if state.denylist().is_revoked(claims.jti).await? {
            return Err(AppError::Unauthorized);
        }

        Ok(AuthUser {
            account_id: claims.sub,
            roles: claims.roles.clone(),
            claims,
        })
    }
}

/// [`AuthUser`] scoped to accounts holding the warehouse role — the handler
/// signature itself documents who may call it, instead of an `if` at the
/// top of the body that's easy to forget on a new route.
#[derive(Debug, Clone)]
pub struct WarehouseUser(pub AuthUser);

#[async_trait]
impl<S> FromRequestParts<S> for WarehouseUser
where
    S: AuthState + Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let user = AuthUser::from_request_parts(parts, state).await?;
        if !user.has_role(ROLE_WAREHOUSE) {
            return Err(AppError::Unauthorized);
        }
        Ok(WarehouseUser(user))
    }
}
