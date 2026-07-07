use axum::{
    Json, Router,
    extract::{Extension, Path, State},
    routing::{get, post},
};
use nvbes_core::http::error::ErrorEnvelope;
use uuid::Uuid;

use super::service;
use super::types::{
    AccessReviewCampaignDetail, AccessReviewCampaignExport, AccessReviewCampaignsResponse,
    AccessReviewDecisionInput, AccessReviewDecisionResponse, CloseAccessReviewCampaignInput,
    CreateAccessReviewCampaignInput,
};
use crate::app::AppState;
use crate::http::error::AppError;
use crate::http::middleware::jwt::AuthContext;

#[path = "identity.domains.enterprise.access_reviews.routes.schedules.rs"]
pub mod schedules;

pub use schedules::{
    create_access_review_schedule, disable_access_review_schedule, enable_access_review_schedule,
    list_access_review_schedules, run_access_review_schedule_now,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/enterprise/access-review-campaigns",
            get(list_access_review_campaigns).post(create_access_review_campaign),
        )
        .route(
            "/enterprise/access-review-schedules",
            get(list_access_review_schedules).post(create_access_review_schedule),
        )
        .route(
            "/enterprise/access-review-schedules/{scheduleId}/disable",
            post(disable_access_review_schedule),
        )
        .route(
            "/enterprise/access-review-schedules/{scheduleId}/enable",
            post(enable_access_review_schedule),
        )
        .route(
            "/enterprise/access-review-schedules/{scheduleId}/run",
            post(run_access_review_schedule_now),
        )
        .route(
            "/enterprise/access-review-campaigns/{campaignId}",
            get(get_access_review_campaign),
        )
        .route(
            "/enterprise/access-review-campaigns/{campaignId}/export",
            get(export_access_review_campaign),
        )
        .route(
            "/enterprise/access-review-campaigns/{campaignId}/close",
            post(close_access_review_campaign),
        )
        .route(
            "/enterprise/access-review-campaigns/{campaignId}/items/{itemId}",
            axum::routing::patch(decide_access_review_item),
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

#[utoipa::path(
    get,
    path = "/enterprise/access-review-campaigns/{campaignId}/export",
    tag = "enterprise",
    params(("campaignId" = Uuid, Path, description = "Access review campaign ID")),
    responses(
        (status = 200, description = "Access review campaign export", body = AccessReviewCampaignExport),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 403, description = "Tenant management denied", body = ErrorEnvelope),
        (status = 404, description = "Campaign not found", body = ErrorEnvelope),
    )
)]
pub async fn export_access_review_campaign(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(campaign_id): Path<Uuid>,
) -> Result<Json<AccessReviewCampaignExport>, AppError> {
    let tenant_id = require_tenant(&auth)?;
    Ok(Json(
        service::export_campaign(&state.db, &auth, tenant_id, campaign_id).await?,
    ))
}

#[utoipa::path(
    post,
    path = "/enterprise/access-review-campaigns/{campaignId}/close",
    tag = "enterprise",
    params(("campaignId" = Uuid, Path, description = "Access review campaign ID")),
    request_body = CloseAccessReviewCampaignInput,
    responses(
        (status = 200, description = "Closed access review campaign", body = AccessReviewCampaignDetail),
        (status = 400, description = "Invalid close input", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 403, description = "Tenant management denied", body = ErrorEnvelope),
        (status = 404, description = "Campaign not found", body = ErrorEnvelope),
        (status = 409, description = "Campaign is already closed", body = ErrorEnvelope),
    )
)]
pub async fn close_access_review_campaign(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(campaign_id): Path<Uuid>,
    Json(input): Json<CloseAccessReviewCampaignInput>,
) -> Result<Json<AccessReviewCampaignDetail>, AppError> {
    let tenant_id = require_tenant(&auth)?;
    Ok(Json(
        service::close_campaign(&state.db, &auth, tenant_id, campaign_id, input).await?,
    ))
}

#[utoipa::path(
    patch,
    path = "/enterprise/access-review-campaigns/{campaignId}/items/{itemId}",
    tag = "enterprise",
    params(
        ("campaignId" = Uuid, Path, description = "Access review campaign ID"),
        ("itemId" = Uuid, Path, description = "Access review item ID")
    ),
    request_body = AccessReviewDecisionInput,
    responses(
        (status = 200, description = "Access review item decision", body = AccessReviewDecisionResponse),
        (status = 400, description = "Invalid decision input", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 403, description = "Tenant management denied", body = ErrorEnvelope),
        (status = 404, description = "Campaign or item not found", body = ErrorEnvelope),
        (status = 409, description = "Campaign is closed", body = ErrorEnvelope),
    )
)]
pub async fn decide_access_review_item(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path((campaign_id, item_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<AccessReviewDecisionInput>,
) -> Result<Json<AccessReviewDecisionResponse>, AppError> {
    let tenant_id = require_tenant(&auth)?;
    Ok(Json(
        service::decide_item(
            &state.db,
            &state.redis,
            &auth,
            tenant_id,
            campaign_id,
            item_id,
            input,
        )
        .await?,
    ))
}

pub(super) fn require_tenant(auth: &AuthContext) -> Result<Uuid, AppError> {
    auth.tenant_id.ok_or_else(|| {
        AppError::bad_request(
            "missing_tenant",
            "Tenant ID is required for enterprise administration.",
        )
    })
}
