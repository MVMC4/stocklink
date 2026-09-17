//! The one error type every service maps to an HTTP response. A handler,
//! service or repository returns [`AppResult<T>`]; [`AppError`] carries just
//! enough to pick a status code and render the error envelope documented in
//! the API reference (`{ "error": { "code", "message", "field"? } }`) —
//! never a raw `sqlx`/`redis` error string, which would leak internals.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// `field`/`message` name exactly what was wrong with the request body,
    /// so the client can point a form error at the right input.
    #[error("validation failed: {field}: {message}")]
    Validation { field: String, message: String },

    /// A malformed or rejected request that isn't a single-field validation
    /// failure (e.g. a downstream service rejected the request as invalid).
    #[error("bad request: {0}")]
    BadRequest(String),

    /// Names *what* wasn't found (e.g. `"order"`), never the id looked up —
    /// the id is either already in the request the client sent, or not
    /// something to echo back.
    #[error("not found: {0}")]
    NotFound(&'static str),

    /// The request is well-formed but conflicts with the resource's current
    /// state (e.g. a duplicate, or a state transition that isn't allowed).
    #[error("conflict: {0}")]
    Conflict(String),

    #[error("unauthorized")]
    Unauthorized,

    #[error("rate limited")]
    RateLimited,

    /// A named dependency isn't usable right now (e.g. `"email delivery"`,
    /// `"identity service"`) — the request itself was fine.
    #[error("service unavailable: {0}")]
    ServiceUnavailable(&'static str),

    /// A documented capability the caller hit before it was built.
    #[error("not implemented: {0}")]
    NotImplemented(&'static str),

    /// Everything that isn't the caller's fault: a bug, a dependency
    /// returning something unexpected, an infrastructure failure. Never
    /// rendered to the client beyond a generic message — logged in full via
    /// `tracing::error!` in [`AppError::into_response`].
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl AppError {
    fn code(&self) -> &'static str {
        match self {
            AppError::Validation { .. } => "VALIDATION",
            AppError::BadRequest(_) => "BAD_REQUEST",
            AppError::NotFound(_) => "NOT_FOUND",
            AppError::Conflict(_) => "CONFLICT",
            AppError::Unauthorized => "UNAUTHORIZED",
            AppError::RateLimited => "RATE_LIMITED",
            AppError::ServiceUnavailable(_) => "SERVICE_UNAVAILABLE",
            AppError::NotImplemented(_) => "NOT_IMPLEMENTED",
            AppError::Internal(_) => "INTERNAL",
        }
    }

    fn status(&self) -> StatusCode {
        match self {
            AppError::Validation { .. } | AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::Unauthorized => StatusCode::UNAUTHORIZED,
            AppError::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            AppError::ServiceUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            AppError::NotImplemented(_) => StatusCode::NOT_IMPLEMENTED,
            AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// The message rendered to the client. [`AppError::Internal`] never
    /// echoes its real cause (that's for `tracing::error!` only) — a bug's
    /// details aren't the caller's business and might contain internals.
    fn public_message(&self) -> String {
        match self {
            AppError::Internal(_) => "an internal error occurred".to_string(),
            other => other.to_string(),
        }
    }
}

#[derive(Serialize)]
struct ErrorBody {
    code: &'static str,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    field: Option<String>,
}

#[derive(Serialize)]
struct ErrorEnvelope {
    error: ErrorBody,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        if let AppError::Internal(err) = &self {
            tracing::error!(error = ?err, "internal error");
        }

        let field = match &self {
            AppError::Validation { field, .. } => Some(field.clone()),
            _ => None,
        };
        let body = ErrorEnvelope {
            error: ErrorBody {
                code: self.code(),
                message: self.public_message(),
                field,
            },
        };
        (self.status(), Json(body)).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => AppError::NotFound("record"),
            other => AppError::Internal(anyhow::Error::new(other).context("database query")),
        }
    }
}

impl From<deadpool_redis::PoolError> for AppError {
    fn from(err: deadpool_redis::PoolError) -> Self {
        AppError::Internal(anyhow::Error::new(err).context("redis connection pool"))
    }
}

impl From<redis::RedisError> for AppError {
    fn from(err: redis::RedisError) -> Self {
        AppError::Internal(anyhow::Error::new(err).context("redis command"))
    }
}
