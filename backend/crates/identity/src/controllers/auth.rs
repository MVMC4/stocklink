//! Email/phone OTP authentication: request a code, verify it, refresh,
//! logout.
//!
//! On success, `verify`/`refresh` set two HttpOnly cookies — `sl_session`
//! (the access token) and `sl_refresh` (the refresh token, scoped to
//! `/v1/auth` only) — so the browser client never touches either token
//! directly (WO-05: no tokens in `localStorage`, including the refresh
//! token). Both are also returned in the JSON body for non-browser clients
//! (the mobile app, `curl`, `cargo test`), which pass `refresh_token`
//! explicitly instead of relying on the cookie.

use axum::extract::{ConnectInfo, State};
use axum::http::HeaderMap;
use axum::Json;
use axum_extra::extract::cookie::{Cookie, SameSite};
use axum_extra::extract::CookieJar;
use std::net::SocketAddr;

use crate::schemas::account::{
    LogoutReq, RefreshReq, RequestOtpReq, RequestOtpRes, TokenPairRes, VerifyOtpReq,
};
use crate::state::AppState;
use stocklink_shared::auth::extractor::SESSION_COOKIE;
use stocklink_shared::auth::AuthUser;
use stocklink_shared::errors::{AppError, AppResult};
use stocklink_shared::utils::validation::ValidatedJson;

/// HttpOnly cookie carrying the refresh token, scoped to `/v1/auth` so it's
/// never sent on ordinary API calls — only on refresh/logout.
const REFRESH_COOKIE: &str = "sl_refresh";
const REFRESH_COOKIE_PATH: &str = "/v1/auth";

#[utoipa::path(
    post,
    path = "/v1/auth/otp/request",
    tag = "auth",
    request_body = RequestOtpReq,
    responses((status = 200, body = RequestOtpRes)),
)]
pub async fn request_otp(
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<RequestOtpReq>,
) -> AppResult<Json<RequestOtpRes>> {
    let ack = state
        .auth
        .request_otp(&req.channel, &req.identifier)
        .await?;
    Ok(Json(RequestOtpRes {
        expires_in_secs: ack.expires_in_secs,
        dev_code: ack.dev_code,
    }))
}

#[utoipa::path(
    post,
    path = "/v1/auth/otp/verify",
    tag = "auth",
    request_body = VerifyOtpReq,
    responses((status = 200, body = TokenPairRes)),
)]
pub async fn verify_otp(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    jar: CookieJar,
    headers: HeaderMap,
    ValidatedJson(req): ValidatedJson<VerifyOtpReq>,
) -> AppResult<(CookieJar, Json<TokenPairRes>)> {
    let user_agent = headers.get("user-agent").and_then(|v| v.to_str().ok());
    let pair = state
        .auth
        .verify_otp(
            &req.channel,
            &req.identifier,
            &req.code,
            user_agent,
            Some(&peer.ip().to_string()),
        )
        .await?;
    let jar = jar
        .add(session_cookie(&pair.access_token, &state))
        .add(refresh_cookie(&pair.refresh_token, &state));
    Ok((
        jar,
        Json(TokenPairRes {
            account_id: pair.account_id,
            access_token: pair.access_token,
            refresh_token: pair.refresh_token,
            roles: pair.roles,
        }),
    ))
}

#[utoipa::path(
    post,
    path = "/v1/auth/refresh",
    tag = "auth",
    request_body = RefreshReq,
    responses((status = 200, body = TokenPairRes)),
)]
pub async fn refresh(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    jar: CookieJar,
    headers: HeaderMap,
    ValidatedJson(req): ValidatedJson<RefreshReq>,
) -> AppResult<(CookieJar, Json<TokenPairRes>)> {
    let refresh_token = req
        .refresh_token
        .or_else(|| jar.get(REFRESH_COOKIE).map(|c| c.value().to_string()))
        .ok_or(AppError::Unauthorized)?;
    let user_agent = headers.get("user-agent").and_then(|v| v.to_str().ok());
    let pair = state
        .auth
        .refresh(&refresh_token, user_agent, Some(&peer.ip().to_string()))
        .await?;
    let jar = jar
        .add(session_cookie(&pair.access_token, &state))
        .add(refresh_cookie(&pair.refresh_token, &state));
    Ok((
        jar,
        Json(TokenPairRes {
            account_id: pair.account_id,
            access_token: pair.access_token,
            refresh_token: pair.refresh_token,
            roles: pair.roles,
        }),
    ))
}

#[utoipa::path(
    post,
    path = "/v1/auth/logout",
    tag = "auth",
    request_body = LogoutReq,
    responses((status = 204)),
)]
pub async fn logout(
    State(state): State<AppState>,
    jar: CookieJar,
    user: Option<AuthUser>,
    ValidatedJson(req): ValidatedJson<LogoutReq>,
) -> AppResult<CookieJar> {
    let refresh_token = req
        .refresh_token
        .or_else(|| jar.get(REFRESH_COOKIE).map(|c| c.value().to_string()));
    let (jti, remaining_ttl) = match &user {
        Some(u) => (
            Some(u.claims.jti),
            u.claims.remaining_ttl_secs(chrono::Utc::now()),
        ),
        None => (None, 0),
    };
    state
        .auth
        .logout(refresh_token.as_deref(), jti, remaining_ttl)
        .await?;
    Ok(jar.remove(Cookie::from(SESSION_COOKIE)).remove(
        Cookie::build(REFRESH_COOKIE)
            .path(REFRESH_COOKIE_PATH)
            .build(),
    ))
}

pub(crate) fn session_cookie(access_token: &str, state: &AppState) -> Cookie<'static> {
    Cookie::build((SESSION_COOKIE, access_token.to_string()))
        .http_only(true)
        .secure(state.config.env.is_production_like())
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(time::Duration::seconds(
            state.config.jwt.access_ttl.as_secs() as i64,
        ))
        .build()
}

pub(crate) fn refresh_cookie(refresh_token: &str, state: &AppState) -> Cookie<'static> {
    Cookie::build((REFRESH_COOKIE, refresh_token.to_string()))
        .http_only(true)
        .secure(state.config.env.is_production_like())
        .same_site(SameSite::Lax)
        .path(REFRESH_COOKIE_PATH)
        .max_age(time::Duration::seconds(
            state.config.jwt.refresh_ttl.as_secs() as i64,
        ))
        .build()
}
