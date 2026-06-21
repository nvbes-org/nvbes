use axum::{
    Json,
    extract::{Extension, Path, State},
};
use nvbes_core::http::error::ErrorEnvelope;
use uuid::Uuid;

use super::{require_tenant, service};
use crate::app::AppState;
use crate::domains::enterprise::access_reviews::types::{
    AccessReviewCampaignDetail, AccessReviewSchedule, AccessReviewSchedulesResponse,
    CreateAccessReviewScheduleInput,
};
use crate::http::error::AppError;
use crate::http::middleware::jwt::AuthContext;

#[utoipa::path(
    get,
    path = "/enterprise/access-review-schedules",
    tag = "enterprise",
    responses(
        (status = 200, description = "Access review schedules", body = AccessReviewSchedulesResponse),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 403, description = "Tenant management denied", body = ErrorEnvelope),
    )
)]
pub async fn list_access_review_schedules(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<AccessReviewSchedulesResponse>, AppError> {
    let tenant_id = require_tenant(&auth)?;
    Ok(Json(
        service::list_schedules(&state.db, &auth, tenant_id).await?,
    ))
}

#[utoipa::path(
    post,
    path = "/enterprise/access-review-schedules",
    tag = "enterprise",
    request_body = CreateAccessReviewScheduleInput,
    responses(
        (status = 200, description = "Created access review schedule", body = AccessReviewSchedule),
        (status = 400, description = "Invalid schedule input", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 403, description = "Tenant management denied", body = ErrorEnvelope),
    )
)]
pub async fn create_access_review_schedule(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(input): Json<CreateAccessReviewScheduleInput>,
) -> Result<Json<AccessReviewSchedule>, AppError> {
    let tenant_id = require_tenant(&auth)?;
    Ok(Json(
        service::create_schedule(&state.db, &auth, tenant_id, input).await?,
    ))
}

#[utoipa::path(
    post,
    path = "/enterprise/access-review-schedules/{scheduleId}/disable",
    tag = "enterprise",
    params(("scheduleId" = Uuid, Path, description = "Access review schedule ID")),
    responses(
        (status = 200, description = "Disabled access review schedule", body = AccessReviewSchedule),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 403, description = "Tenant management denied", body = ErrorEnvelope),
        (status = 404, description = "Schedule not found", body = ErrorEnvelope),
    )
)]
pub async fn disable_access_review_schedule(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(schedule_id): Path<Uuid>,
) -> Result<Json<AccessReviewSchedule>, AppError> {
    let tenant_id = require_tenant(&auth)?;
    Ok(Json(
        service::disable_schedule(&state.db, &auth, tenant_id, schedule_id).await?,
    ))
}

#[utoipa::path(
    post,
    path = "/enterprise/access-review-schedules/{scheduleId}/enable",
    tag = "enterprise",
    params(("scheduleId" = Uuid, Path, description = "Access review schedule ID")),
    responses(
        (status = 200, description = "Enabled access review schedule", body = AccessReviewSchedule),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 403, description = "Tenant management denied", body = ErrorEnvelope),
        (status = 404, description = "Schedule not found", body = ErrorEnvelope),
    )
)]
pub async fn enable_access_review_schedule(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(schedule_id): Path<Uuid>,
) -> Result<Json<AccessReviewSchedule>, AppError> {
    let tenant_id = require_tenant(&auth)?;
    Ok(Json(
        service::enable_schedule(&state.db, &auth, tenant_id, schedule_id).await?,
    ))
}

#[utoipa::path(
    post,
    path = "/enterprise/access-review-schedules/{scheduleId}/run",
    tag = "enterprise",
    params(("scheduleId" = Uuid, Path, description = "Access review schedule ID")),
    responses(
        (status = 200, description = "Created access review campaign from schedule", body = AccessReviewCampaignDetail),
        (status = 400, description = "Empty access review scope", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 403, description = "Tenant management denied", body = ErrorEnvelope),
        (status = 404, description = "Schedule not found", body = ErrorEnvelope),
        (status = 409, description = "Schedule is disabled", body = ErrorEnvelope),
    )
)]
pub async fn run_access_review_schedule_now(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(schedule_id): Path<Uuid>,
) -> Result<Json<AccessReviewCampaignDetail>, AppError> {
    let tenant_id = require_tenant(&auth)?;
    Ok(Json(
        service::run_schedule_now(&state.db, &auth, tenant_id, schedule_id).await?,
    ))
}
