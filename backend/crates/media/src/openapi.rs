//! OpenAPI document assembly: every `#[utoipa::path]`-annotated handler and
//! the schema it references, listed once here rather than discovered by
//! macro magic — the OpenAPI coverage guard (WO-03) diffs this list against
//! the mounted routes, so an entry only mounted here and not in `routes.rs`
//! (or vice versa) fails CI instead of silently drifting.

use utoipa::OpenApi;

use crate::controllers::media;
use crate::schemas::common::{DependencyStatus, ErrorBody, ErrorRes, ReadyRes};
use crate::schemas::media::{PresignUploadReq, PresignUploadRes};

#[derive(OpenApi)]
#[openapi(
    paths(media::presign_upload,),
    components(schemas(
        PresignUploadReq,
        PresignUploadRes,
        DependencyStatus,
        ReadyRes,
        ErrorBody,
        ErrorRes,
    )),
    tags((name = "media", description = "Presigned uploads for catalogue photos and proof of delivery")),
)]
pub struct ApiDoc;
