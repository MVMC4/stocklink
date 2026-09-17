//! Response shapes shared across more than one controller in this service:
//! health-check status and the documented error envelope. Domain-specific
//! request/response types live in their own `schemas::<domain>` module.

use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct DependencyStatus {
    pub name: &'static str,
    pub healthy: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ReadyRes {
    pub status: &'static str,
    pub dependencies: Vec<DependencyStatus>,
}

impl ReadyRes {
    pub fn from_dependencies(dependencies: Vec<DependencyStatus>) -> Self {
        let status = if dependencies.iter().all(|d| d.healthy) {
            "ok"
        } else {
            "degraded"
        };
        Self {
            status,
            dependencies,
        }
    }
}

/// Mirrors the error envelope every non-2xx response actually uses (see
/// `stocklink_shared::errors::AppError`) — this type exists purely so the
/// OpenAPI document can describe that shape; nothing constructs it at
/// runtime.
#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorRes {
    pub error: ErrorBody,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
}
