use chrono::Utc;
use uuid::Uuid;

#[path = "identity.domains.enterprise.access_reviews.service.schedules.rs"]
mod service_schedules;

pub use service_schedules::{
    create_schedule, disable_schedule, enable_schedule, enqueue_due_campaign_reminders,
    list_schedules, materialize_due_schedules, run_schedule_now,
};

use super::{changes, db, decisions, revocations, runtime, types::*, validation};
use crate::database::Database;
use crate::domains::authz::{AdminScope, resolve_admin_scope};
use crate::domains::enterprise::service::require_actor_access_for_enterprise;
use crate::http::error::AppError;
use crate::http::middleware::jwt::AuthContext;

pub(super) async fn ensure_tenant_admin(
    db: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
) -> Result<(), AppError> {
    crate::domains::authz::ensure_tenant_management_access(db, auth, tenant_id).await?;
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
    ensure_tenant_admin(db, auth, tenant_id).await?;
    campaign_detail(db, tenant_id, campaign_id).await
}

pub async fn export_campaign(
    db: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
    campaign_id: Uuid,
) -> Result<AccessReviewCampaignExport, AppError> {
    let detail = get_campaign(db, auth, tenant_id, campaign_id).await?;
    Ok(AccessReviewCampaignExport {
        campaign: detail.campaign,
        generated_at: Utc::now(),
        rows: detail
            .items
            .into_iter()
            .map(|item| AccessReviewCampaignExportRow {
                item_id: item.id,
                item_type: item.item_type,
                subject_id: item.subject_id,
                subject_label: item.subject_label,
                workspace_id: item.workspace_id,
                role: item.role,
                status: item.status,
                decision: item.decision,
                reviewed_by: item.reviewed_by,
                reviewed_at: item.reviewed_at,
                created_at: item.created_at,
                evidence: item.evidence,
            })
            .collect(),
    })
}

pub async fn create_campaign(
    db: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
    input: CreateAccessReviewCampaignInput,
) -> Result<AccessReviewCampaignDetail, AppError> {
    ensure_tenant_admin(db, auth, tenant_id).await?;
    validation::validate_campaign_input(&input)?;

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

pub async fn close_campaign(
    db: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
    campaign_id: Uuid,
    input: CloseAccessReviewCampaignInput,
) -> Result<AccessReviewCampaignDetail, AppError> {
    ensure_tenant_admin(db, auth, tenant_id).await?;

    let mut tx = db.begin().await?;
    let campaign = decisions::reviewable_campaign(&mut tx, tenant_id, campaign_id)
        .await?
        .ok_or_else(|| {
            AppError::not_found(
                "access_review_campaign_not_found",
                "Access review campaign not found.",
            )
        })?;
    if campaign.status != "active" {
        return Err(AppError::conflict(
            "access_review_campaign_already_closed",
            "Only active access review campaigns can be closed.",
        ));
    }

    let pending_items = decisions::pending_item_count(&mut tx, tenant_id, campaign_id).await?;
    validation::validate_close_input(&input, pending_items)?;
    let note = input
        .note
        .as_deref()
        .map(str::trim)
        .filter(|note| !note.is_empty());
    decisions::close_campaign(&mut tx, tenant_id, campaign_id).await?;
    crate::domains::enterprise::db::insert_audit(
        &mut tx,
        tenant_id,
        auth.user_id,
        "enterprise.access_review_campaign.closed",
        "access_review_campaign",
        Some(campaign_id),
        serde_json::json!({
            "pending_items": pending_items,
            "note": note
        }),
    )
    .await?;
    tx.commit().await?;

    campaign_detail(db, tenant_id, campaign_id).await
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

    let mut tx = db.begin().await?;
    let campaign = decisions::reviewable_campaign(&mut tx, tenant_id, campaign_id)
        .await?
        .ok_or_else(|| {
            AppError::not_found(
                "access_review_campaign_not_found",
                "Access review campaign not found.",
            )
        })?;
    if campaign.status != "active" {
        return Err(AppError::conflict(
            "access_review_campaign_closed",
            "Only active access review campaigns can be updated.",
        ));
    }

    let decision = validation::decision_as_db(&input.decision);
    let existing_item = decisions::get_item_for_update(&mut tx, tenant_id, campaign_id, item_id)
        .await?
        .ok_or_else(|| {
            AppError::not_found(
                "access_review_item_not_found",
                "Access review item not found.",
            )
        })?;
    if existing_item.decision != "pending" {
        return Err(AppError::conflict(
            "access_review_item_already_decided",
            "This access review item already has a decision.",
        ));
    }
    let applied_revocation = if matches!(input.decision, AccessReviewItemDecision::Revoked) {
        Some(revocations::apply_revocation(&mut tx, tenant_id, &existing_item).await?)
    } else {
        None
    };
    let applied_change = if matches!(input.decision, AccessReviewItemDecision::Changed) {
        Some(changes::apply_change(&mut tx, tenant_id, &existing_item, &input).await?)
    } else {
        None
    };
    if let Some(oauth_client) = applied_revocation
        .as_ref()
        .and_then(|revocation| revocation.oauth_client.as_ref())
    {
        runtime::revoke_oauth_client(redis, oauth_client).await?;
    }
    let item = decisions::update_item_decision(
        &mut tx,
        tenant_id,
        campaign_id,
        item_id,
        auth.user_id,
        decision,
        input.note.as_deref().map(str::trim),
    )
    .await?
    .ok_or_else(|| {
        AppError::not_found(
            "access_review_item_not_found",
            "Access review item not found.",
        )
    })?;

    let pending_items = decisions::pending_item_count(&mut tx, tenant_id, campaign_id).await?;
    if pending_items == 0 {
        decisions::close_campaign(&mut tx, tenant_id, campaign_id).await?;
    }

    crate::domains::enterprise::db::insert_audit(
        &mut tx,
        tenant_id,
        auth.user_id,
        "enterprise.access_review_item.decided",
        "access_review_item",
        Some(item.id),
        serde_json::json!({
            "campaign_id": campaign_id,
            "decision": decision,
            "note": input.note,
            "applied_revocation": applied_revocation.as_ref().map(|revocation| serde_json::json!({
                "target_type": revocation.target_type,
                "target_id": revocation.target_id,
            })),
            "applied_change": applied_change.as_ref().map(|change| serde_json::json!({
                "target_type": change.target_type,
                "target_id": change.target_id,
                "target_role": change.target_role,
            }))
        }),
    )
    .await?;
    tx.commit().await?;

    let campaign = db::get_campaign(db, tenant_id, campaign_id)
        .await?
        .ok_or_else(|| {
            AppError::not_found(
                "access_review_campaign_not_found",
                "Access review campaign not found.",
            )
        })?
        .into_view();
    Ok(AccessReviewDecisionResponse {
        campaign,
        item: item.into_view(),
    })
}

pub(super) async fn campaign_detail(
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
