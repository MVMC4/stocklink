//! OpenAPI document assembly: every `#[utoipa::path]`-annotated handler and
//! the schema it references, listed once here rather than discovered by
//! macro magic — the OpenAPI coverage guard (WO-03) diffs this list against
//! the mounted routes, so an entry only mounted here and not in `routes.rs`
//! (or vice versa) fails CI instead of silently drifting.
//!
//! No handler in this service is annotated with `#[utoipa::path]` yet
//! (`controllers::notifications` predates that convention being applied
//! here) — the document below is deliberately paths-empty rather than
//! guessed, per AGENTS.md rule 4.

use utoipa::OpenApi;

use crate::schemas::common::{DependencyStatus, ErrorBody, ErrorRes, ReadyRes};
use crate::schemas::notifications::NotificationRes;

#[derive(OpenApi)]
#[openapi(
    paths(),
    components(schemas(
        NotificationRes,
        DependencyStatus,
        ReadyRes,
        ErrorBody,
        ErrorRes,
    )),
    tags((name = "notifications", description = "In-app notification inbox and device registration")),
)]
pub struct ApiDoc;
