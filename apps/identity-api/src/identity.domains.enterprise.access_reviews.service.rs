use chrono::Utc;
use uuid::Uuid;

use super::{db, types::*};
use crate::database::Database;
use crate::domains::enterprise::service::require_actor_access_for_enterprise;
use crate::http::error::AppError;
use crate::http::middleware::jwt::AuthContext;

pub async fn list_campaigns(
    db: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
) -> Result<AccessReviewCampaignsResponse, AppError> {
    crate::domains::authz::ensure_tenant_management_access(db, auth, tenant_id).await?;
    require_actor_access_for_enterprise(db, auth, tenant_id).await?;
    Ok(AccessReviewCampaignsResponse {
        campaigns: db::list_campaigns(db, tenant_id)
            .await?
            .into_iter()
            .map(db::AccessReviewCampaignRow::into_view)
            .collect(),
    })
}

pub async fn get_campaign(
    db: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
    campaign_id: Uuid,
) -> Result<AccessReviewCampaignDetail, AppError> {
    crate::domains::authz::ensure_tenant_management_access(db, auth, tenant_id).await?;
    require_actor_access_for_enterprise(db, auth, tenant_id).await?;
    campaign_detail(db, tenant_id, campaign_id).await
}

pub async fn create_campaign(
    db: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
    input: CreateAccessReviewCampaignInput,
) -> Result<AccessReviewCampaignDetail, AppError> {
    crate::domains::authz::ensure_tenant_management_access(db, auth, tenant_id).await?;
    require_actor_access_for_enterprise(db, auth, tenant_id).await?;
    validate_campaign_input(&input)?;

    let mut tx = db.begin().await?;
    let campaign_id = db::insert_campaign(
        &mut tx,
        tenant_id,
        auth.user_id,
        input.name.trim(),
        input.description.as_deref().map(str::trim),
        input.due_at,
    )
    .await?;
    let item_count =
        db::insert_snapshot_items(&mut tx, campaign_id, tenant_id, &input.scope).await?;
    if item_count == 0 {
        return Err(AppError::bad_request(
            "empty_access_review_scope",
            "The selected access review scope does not contain any reviewable access.",
        ));
    }
    crate::domains::enterprise::db::insert_audit(
        &mut tx,
        tenant_id,
        auth.user_id,
        "enterprise.access_review_campaign.created",
        "access_review_campaign",
        Some(campaign_id),
        serde_json::json!({
            "name": input.name.trim(),
            "due_at": input.due_at,
            "item_count": item_count
        }),
    )
    .await?;
    tx.commit().await?;
    campaign_detail(db, tenant_id, campaign_id).await
}

async fn campaign_detail(
    db: &Database,
    tenant_id: Uuid,
    campaign_id: Uuid,
) -> Result<AccessReviewCampaignDetail, AppError> {
    let campaign = db::get_campaign(db, tenant_id, campaign_id)
        .await?
        .ok_or_else(|| {
            AppError::not_found(
                "access_review_campaign_not_found",
                "Access review campaign not found.",
            )
        })?
        .into_view();
    let items = db::list_items(db, tenant_id, campaign_id)
        .await?
        .into_iter()
        .map(db::AccessReviewItemRow::into_view)
        .collect();
    Ok(AccessReviewCampaignDetail { campaign, items })
}

fn validate_campaign_input(input: &CreateAccessReviewCampaignInput) -> Result<(), AppError> {
    if input.name.trim().is_empty() {
        return Err(AppError::bad_request(
            "validation_failed",
            "Access review campaign name is required.",
        ));
    }
    if input.due_at <= Utc::now() {
        return Err(AppError::bad_request(
            "validation_failed",
            "Access review campaign due date must be in the future.",
        ));
    }
    if !input.scope.include_members
        && !input.scope.include_roles
        && !input.scope.include_service_accounts
        && !input.scope.include_oauth_clients
    {
        return Err(AppError::bad_request(
            "validation_failed",
            "At least one access review scope must be selected.",
        ));
    }
    Ok(())
}
