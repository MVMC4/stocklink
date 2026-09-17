//! Request-body validation and typed path parameters. Body validation lives
//! here, not in each handler, so a rejected request always renders the same
//! `{ "error": { "code": "VALIDATION", "field", "message" } }` shape (see
//! the API reference's "Error envelope" section) instead of every handler
//! reinventing its own.

use async_trait::async_trait;
use axum::extract::{FromRequest, FromRequestParts, Path, Request};
use axum::http::request::Parts;
use axum::Json;
use serde::de::DeserializeOwned;
use uuid::Uuid;
use validator::Validate;

use crate::errors::AppError;

/// A JSON body, deserialized then validated. Rejects with
/// [`AppError::Validation`], naming the *first* failing field — enough for
/// a client to point a form error at the right input; a full multi-field
/// report isn't part of the documented error envelope.
pub struct ValidatedJson<T>(pub T);

#[async_trait]
impl<T, S> FromRequest<S> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state)
            .await
            .map_err(|err| AppError::BadRequest(err.to_string()))?;

        if let Err(errors) = value.validate() {
            let (field, message) = first_error(&errors);
            return Err(AppError::Validation { field, message });
        }

        Ok(ValidatedJson(value))
    }
}

fn first_error(errors: &validator::ValidationErrors) -> (String, String) {
    let field_errors = errors.field_errors();
    let mut fields: Vec<_> = field_errors.keys().collect();
    fields.sort();

    match fields.first() {
        Some(field) => {
            let message = field_errors[**field]
                .first()
                .map(|e| {
                    e.message
                        .clone()
                        .map(|m| m.to_string())
                        .unwrap_or_else(|| format!("failed `{}` validation", e.code))
                })
                .unwrap_or_else(|| "invalid value".to_string());
            (field.to_string(), message)
        }
        None => ("unknown".to_string(), "validation failed".to_string()),
    }
}

/// A single `Uuid` path parameter, rejecting a malformed one as a plain
/// `AppError::BadRequest` rather than the framework's default plaintext
/// 400 — so it's still rendered through the standard error envelope.
pub struct UuidPath(pub Uuid);

#[async_trait]
impl<S> FromRequestParts<S> for UuidPath
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Path(id) = Path::<Uuid>::from_request_parts(parts, state)
            .await
            .map_err(|_| AppError::BadRequest("invalid id in path".to_string()))?;
        Ok(UuidPath(id))
    }
}

/// Two `Uuid` path parameters, in path order (e.g.
/// `/warehouses/{warehouse_id}/orders/{order_id}`).
pub struct UuidPath2(pub Uuid, pub Uuid);

#[async_trait]
impl<S> FromRequestParts<S> for UuidPath2
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Path((a, b)) = Path::<(Uuid, Uuid)>::from_request_parts(parts, state)
            .await
            .map_err(|_| AppError::BadRequest("invalid id in path".to_string()))?;
        Ok(UuidPath2(a, b))
    }
}
