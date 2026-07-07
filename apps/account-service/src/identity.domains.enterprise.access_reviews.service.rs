use uuid::Uuid;

#[path = "identity.domains.enterprise.access_reviews.service.export.rs"]
mod export;
#[path = "identity.domains.enterprise.access_reviews.service.enterprise.rs"]
mod service_enterprise;
#[path = "identity.domains.enterprise.access_reviews.service.schedules.rs"]
mod service_schedules;

pub use export::export_campaign;
pub use service_schedules::{
    create_schedule, disable_schedule, enable_schedule, list_schedules, materialize_due_schedules,
    run_schedule_now,
};

use super::{types::*, validation};
use crate::database::Database;
use crate::domains::authz::{AdminScope, resolve_admin_scope};
use crate::domains::enterprise::grpc as enterprise_grpc;
use crate::domains::enterprise::service::require_actor_access_for_enterprise;
use crate::http::error::AppError;
use crate::http::middleware::jwt::AuthContext;

pub(super) async fn ensure_tenant_admin(
    db: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
) -> Result<(), AppError> {
    let scope = resolve_admin_scope(db, auth, tenant_id, auth.organization_id).await?;
    if !matches!(scope, AdminScope::Tenant) {
        return Err(AppError::forbidden(
            "tenant_scope_required",
            "This action requires tenant-wide administrative privileges.",
        ));
    }
    require_actor_access_for_enterprise(db, auth, tenant_id).await?;
    Ok(())
}

pub async fn list_campaigns(
    db: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
) -> Result<AccessReviewCampaignsResponse, AppError> {
    ensure_tenant_admin(db, auth, tenant_id).await?;
    enterprise_grpc::access_reviews::list_campaigns(tenant_id, auth.user_id).await
}

pub async fn get_campaign(
    db: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
    campaign_id: Uuid,
) -> Result<AccessReviewCampaignDetail, AppError> {
    ensure_tenant_admin(db, auth, tenant_id).await?;
    campaign_detail(tenant_id, auth.user_id, campaign_id).await
}

pub async fn create_campaign(
    db: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
    input: CreateAccessReviewCampaignInput,
) -> Result<AccessReviewCampaignDetail, AppError> {
    ensure_tenant_admin(db, auth, tenant_id).await?;
    validation::validate_campaign_input(&input)?;

    let campaign_id =
        enterprise_grpc::access_reviews::start_access_review(tenant_id, auth.user_id, &input)
            .await?;
    campaign_detail(tenant_id, auth.user_id, campaign_id).await
}

pub async fn close_campaign(
    db: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
    campaign_id: Uuid,
    input: CloseAccessReviewCampaignInput,
) -> Result<AccessReviewCampaignDetail, AppError> {
    ensure_tenant_admin(db, auth, tenant_id).await?;
    let detail = campaign_detail(tenant_id, auth.user_id, campaign_id).await?;
    if !matches!(detail.campaign.status, AccessReviewCampaignStatus::Active) {
        return Err(AppError::conflict(
            "access_review_campaign_already_closed",
            "Only active access review campaigns can be closed.",
        ));
    }

    let pending_items = detail.campaign.pending_items;
    validation::validate_close_input(&input, pending_items)?;
    enterprise_grpc::access_reviews::close_access_review(
        tenant_id,
        auth.user_id,
        campaign_id,
        &input,
    )
    .await?;

    campaign_detail(tenant_id, auth.user_id, campaign_id).await
}

pub async fn decide_item(
    db: &Database,
    redis: &nvbes_redis::RedisPool,
    auth: &AuthContext,
    tenant_id: Uuid,
    campaign_id: Uuid,
    item_id: Uuid,
    input: AccessReviewDecisionInput,
) -> Result<AccessReviewDecisionResponse, AppError> {
    ensure_tenant_admin(db, auth, tenant_id).await?;
    validation::validate_decision_input(&input)?;
    service_enterprise::decide_item(redis, auth, tenant_id, campaign_id, item_id, input).await
}

pub(super) async fn campaign_detail(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    campaign_id: Uuid,
) -> Result<AccessReviewCampaignDetail, AppError> {
    enterprise_grpc::access_reviews::get_campaign(tenant_id, actor_principal_id, campaign_id).await
}
