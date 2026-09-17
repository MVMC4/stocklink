//! OpenAPI document assembly: every `#[utoipa::path]`-annotated handler and
//! the schema it references, listed once here rather than discovered by
//! macro magic — the OpenAPI coverage guard (WO-03) diffs this list against
//! the mounted routes, so an entry only mounted here and not in `routes.rs`
//! (or vice versa) fails CI instead of silently drifting.

use utoipa::OpenApi;

use crate::controllers::{account, admin, auth};
use crate::schemas::account::{
    CarrierRes, LogoutReq, RefreshReq, RegisterCarrierReq, RegisterStoreReq, RegisterWarehouseReq,
    RequestOtpReq, RequestOtpRes, StoreRes, TokenPairRes, VerifyOtpReq, WarehouseRes,
};
use crate::schemas::admin::{AdminAuditEntryRes, AdminLinksRes};
use crate::schemas::common::{DependencyStatus, ErrorBody, ErrorRes, ReadyRes};

#[derive(OpenApi)]
#[openapi(
    paths(
        auth::request_otp,
        auth::verify_otp,
        auth::refresh,
        auth::logout,
        account::register_warehouse,
        account::register_store,
        account::register_carrier,
        admin::audit_log,
        admin::links,
    ),
    components(schemas(
        RequestOtpReq,
        RequestOtpRes,
        VerifyOtpReq,
        RefreshReq,
        LogoutReq,
        TokenPairRes,
        RegisterWarehouseReq,
        WarehouseRes,
        RegisterStoreReq,
        StoreRes,
        RegisterCarrierReq,
        CarrierRes,
        AdminAuditEntryRes,
        AdminLinksRes,
        DependencyStatus,
        ReadyRes,
        ErrorBody,
        ErrorRes,
    )),
    tags(
        (name = "auth", description = "Email/phone OTP sign-in, refresh and logout"),
        (name = "onboarding", description = "Warehouse, store and carrier registration"),
        (name = "admin", description = "Admin console: audit log and deep links"),
    ),
)]
pub struct ApiDoc;
