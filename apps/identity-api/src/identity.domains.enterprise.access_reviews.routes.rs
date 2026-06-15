use axum::{
    Json, Router,
    extract::{Extension, Path, State},
    routing::get,
};
use nvbes_core::http::error::ErrorEnvelope;
use uuid::Uuid;

use super::service;
use super::types::{
    AccessReviewCampaignDetail, AccessReviewCampaignsResponse, CreateAccessReviewCampaignInput,
};
use crate::app::AppState;
use crate::http::error::AppError;
use crate::http::middleware::jwt::AuthContext;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/enterprise/access-review-campaigns",
            get(list_access_review_campaigns).post(create_access_review_campaign),
        )
        .route(
            "/enterprise/access-review-campaigns/{campaignId}",
            get(get_access_review_campaign),
        )
}

#[utoipa::path(
    get,
    path = "/enterprise/access-review-campaigns",
    tag = "enterprise",
    responses(
        (status = 200, description = "Access review campaigns", body = AccessReviewCampaignsResponse),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 403, description = "Tenant management denied", body = ErrorEnvelope),
    )
)]
pub async fn list_access_review_campaigns(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<AccessReviewCampaignsResponse>, AppError> {
    let tenant_id = require_tenant(&auth)?;
    Ok(Json(
        service::list_campaigns(&state.db, &auth, tenant_id).await?,
    ))
}

#[utoipa::path(
    post,
    path = "/enterprise/access-review-campaigns",
    tag = "enterprise",
    request_body = CreateAccessReviewCampaignInput,
    responses(
        (status = 200, description = "Created access review campaign", body = AccessReviewCampaignDetail),
        (status = 400, description = "Invalid campaign input", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 403, description = "Tenant management denied", body = ErrorEnvelope),
    )
)]
pub async fn create_access_review_campaign(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(input): Json<CreateAccessReviewCampaignInput>,
) -> Result<Json<AccessReviewCampaignDetail>, AppError> {
    let tenant_id = require_tenant(&auth)?;
    Ok(Json(
        service::create_campaign(&state.db, &auth, tenant_id, input).await?,
    ))
}

#[utoipa::path(
    get,
    path = "/enterprise/access-review-campaigns/{campaignId}",
    tag = "enterprise",
    params(("campaignId" = Uuid, Path, description = "Access review campaign ID")),
    responses(
        (status = 200, description = "Access review campaign detail", body = AccessReviewCampaignDetail),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 403, description = "Tenant management denied", body = ErrorEnvelope),
        (status = 404, description = "Campaign not found", body = ErrorEnvelope),
    )
)]
pub async fn get_access_review_campaign(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(campaign_id): Path<Uuid>,
) -> Result<Json<AccessReviewCampaignDetail>, AppError> {
    let tenant_id = require_tenant(&auth)?;
    Ok(Json(
        service::get_campaign(&state.db, &auth, tenant_id, campaign_id).await?,
    ))
}

fn require_tenant(auth: &AuthContext) -> Result<Uuid, AppError> {
    auth.tenant_id.ok_or_else(|| {
        AppError::bad_request(
            "missing_tenant",
            "Tenant ID is required for enterprise administration.",
        )
    })
}
