use chrono::{DateTime, Utc};
use std::collections::BTreeMap;
use uuid::Uuid;

use crate::{
    domains::enterprise::access_reviews::types::{
        AccessReviewCampaignDetail, AccessReviewCampaignScopeInput, AccessReviewCampaignStatus,
        AccessReviewCampaignSummary, AccessReviewCampaignsResponse, AccessReviewDecisionInput,
        AccessReviewItem, AccessReviewItemDecision, AccessReviewItemType,
        CloseAccessReviewCampaignInput, CreateAccessReviewCampaignInput,
    },
    grpc_pb::nvbes::enterprise::v1::{
        self as enterprise, AccessReviewDetail as GrpcAccessReviewDetail,
        AccessReviewItem as GrpcAccessReviewItem, AccessReviewSummary as GrpcAccessReviewSummary,
        CloseAccessReviewRequest, GetAccessReviewRequest, ListAccessReviewsRequest,
        RecordAccessReviewDecisionRequest, StartAccessReviewRequest,
    },
    http::error::AppError,
};

#[path = "identity.domains.enterprise.grpc.access_reviews.schedules.rs"]
pub mod schedules;

pub async fn list_campaigns(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
) -> Result<AccessReviewCampaignsResponse, AppError> {
    let mut client = super::enterprise_client().await?;
    let response = client
        .list_access_reviews(ListAccessReviewsRequest {
            context: Some(super::request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
        })
        .await
        .map_err(super::grpc_error)?
        .into_inner();
    Ok(AccessReviewCampaignsResponse {
        campaigns: response
            .campaigns
            .into_iter()
            .map(summary_from_grpc)
            .collect::<Result<_, _>>()?,
    })
}

pub async fn get_campaign(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    campaign_id: Uuid,
) -> Result<AccessReviewCampaignDetail, AppError> {
    let mut client = super::enterprise_client().await?;
    let response = client
        .get_access_review(GetAccessReviewRequest {
            context: Some(super::request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
            review_id: campaign_id.to_string(),
        })
        .await
        .map_err(super::grpc_error)?
        .into_inner();
    detail_from_grpc(response)
}

pub async fn start_access_review(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    input: &CreateAccessReviewCampaignInput,
) -> Result<Uuid, AppError> {
    let mut client = super::enterprise_client().await?;
    let response = client
        .start_access_review(StartAccessReviewRequest {
            context: Some(super::request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
            name: input.name.clone(),
            description: input.description.clone().unwrap_or_default(),
            scope: access_review_scope_label(&input.scope).to_string(),
            due_at: input.due_at.to_rfc3339(),
            include_members: input.scope.include_members,
            include_roles: input.scope.include_roles,
            include_service_accounts: input.scope.include_service_accounts,
            include_oauth_clients: input.scope.include_oauth_clients,
        })
        .await
        .map_err(super::grpc_error)?
        .into_inner();
    Uuid::parse_str(&response.review_id).map_err(|error| {
        AppError::internal(
            "enterprise_grpc_invalid_access_review_id",
            format!("Enterprise gRPC returned an invalid access review ID: {error}"),
        )
    })
}

pub async fn record_access_review_decision(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    campaign_id: Uuid,
    item_id: Uuid,
    input: &AccessReviewDecisionInput,
) -> Result<enterprise::AccessReviewDecision, AppError> {
    let mut client = super::enterprise_client().await?;
    let response = client
        .record_access_review_decision(RecordAccessReviewDecisionRequest {
            context: Some(super::request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
            review_id: campaign_id.to_string(),
            item_id: item_id.to_string(),
            subject_principal_id: String::new(),
            decision: access_review_decision_label(&input.decision).to_string(),
            reason: input.note.clone().unwrap_or_default(),
            target_role: input
                .change
                .as_ref()
                .and_then(|change| change.target_role.clone())
                .unwrap_or_default(),
        })
        .await
        .map_err(super::grpc_error)?;
    Ok(response.into_inner())
}

pub async fn close_access_review(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    campaign_id: Uuid,
    input: &CloseAccessReviewCampaignInput,
) -> Result<(), AppError> {
    let mut client = super::enterprise_client().await?;
    client
        .close_access_review(CloseAccessReviewRequest {
            context: Some(super::request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
            review_id: campaign_id.to_string(),
            reason: input.note.clone().unwrap_or_default(),
        })
        .await
        .map_err(super::grpc_error)?;
    Ok(())
}

fn detail_from_grpc(value: GrpcAccessReviewDetail) -> Result<AccessReviewCampaignDetail, AppError> {
    let campaign = value.campaign.ok_or_else(|| {
        AppError::internal(
            "enterprise_grpc_invalid_access_review",
            "Enterprise gRPC returned an access review detail without a campaign.",
        )
    })?;
    Ok(AccessReviewCampaignDetail {
        campaign: summary_from_grpc(campaign)?,
        items: value
            .items
            .into_iter()
            .map(item_from_grpc)
            .collect::<Result<_, _>>()?,
    })
}

fn summary_from_grpc(
    value: GrpcAccessReviewSummary,
) -> Result<AccessReviewCampaignSummary, AppError> {
    Ok(AccessReviewCampaignSummary {
        id: parse_uuid(&value.review_id, "review_id")?,
        name: value.name,
        description: optional_text(value.description),
        status: campaign_status(&value.status),
        starts_at: parse_time(&value.starts_at, "starts_at")?,
        due_at: parse_time(&value.due_at, "due_at")?,
        created_by: parse_uuid(&value.created_by, "created_by")?,
        created_at: parse_time(&value.created_at, "created_at")?,
        closed_at: optional_time(&value.closed_at, "closed_at")?,
        pending_items: value.pending_items,
        approved_items: value.approved_items,
        revoked_items: value.revoked_items,
        changed_items: value.changed_items,
        due_soon_reminders_sent: value.due_soon_reminders_sent,
        overdue_reminders_sent: value.overdue_reminders_sent,
        last_reminder_at: optional_time(&value.last_reminder_at, "last_reminder_at")?,
    })
}

fn item_from_grpc(value: GrpcAccessReviewItem) -> Result<AccessReviewItem, AppError> {
    Ok(AccessReviewItem {
        id: parse_uuid(&value.item_id, "item_id")?,
        item_type: item_type(&value.item_type),
        subject_id: value.subject_id,
        subject_label: value.subject_label,
        workspace_id: optional_uuid(&value.workspace_id, "workspace_id")?,
        role: optional_text(value.role),
        status: value.status,
        evidence: serde_json::from_str::<BTreeMap<String, serde_json::Value>>(&value.evidence_json)
            .map_err(|error| {
                AppError::internal(
                    "enterprise_grpc_invalid_access_review",
                    format!("Enterprise gRPC returned invalid access review evidence: {error}"),
                )
            })?,
        decision: decision(&value.decision),
        reviewed_by: optional_uuid(&value.reviewed_by, "reviewed_by")?,
        reviewed_at: optional_time(&value.reviewed_at, "reviewed_at")?,
        created_at: parse_time(&value.created_at, "created_at")?,
    })
}

fn campaign_status(value: &str) -> AccessReviewCampaignStatus {
    match value {
        "draft" => AccessReviewCampaignStatus::Draft,
        "closed" => AccessReviewCampaignStatus::Closed,
        _ => AccessReviewCampaignStatus::Active,
    }
}

fn item_type(value: &str) -> AccessReviewItemType {
    match value {
        "role" => AccessReviewItemType::Role,
        "service_account" => AccessReviewItemType::ServiceAccount,
        "oauth_client" => AccessReviewItemType::OAuthClient,
        _ => AccessReviewItemType::Member,
    }
}

fn decision(value: &str) -> AccessReviewItemDecision {
    match value {
        "approved" => AccessReviewItemDecision::Approved,
        "revoked" => AccessReviewItemDecision::Revoked,
        "changed" => AccessReviewItemDecision::Changed,
        _ => AccessReviewItemDecision::Pending,
    }
}

fn access_review_scope_label(scope: &AccessReviewCampaignScopeInput) -> &'static str {
    match (
        scope.include_members,
        scope.include_roles,
        scope.include_service_accounts,
        scope.include_oauth_clients,
    ) {
        (true, true, true, true) => "all",
        (true, false, false, false) => "members",
        (false, true, false, false) => "roles",
        (false, false, true, false) => "service_accounts",
        (false, false, false, true) => "oauth_clients",
        _ => "custom",
    }
}

fn access_review_decision_label(decision: &AccessReviewItemDecision) -> &'static str {
    match decision {
        AccessReviewItemDecision::Approved => "approved",
        AccessReviewItemDecision::Revoked => "revoked",
        AccessReviewItemDecision::Changed => "changed",
        AccessReviewItemDecision::Pending => "pending",
    }
}

fn parse_uuid(value: &str, field: &'static str) -> Result<Uuid, AppError> {
    Uuid::parse_str(value.trim()).map_err(|error| invalid_field(field, error))
}

fn optional_uuid(value: &str, field: &'static str) -> Result<Option<Uuid>, AppError> {
    if value.trim().is_empty() {
        Ok(None)
    } else {
        parse_uuid(value, field).map(Some)
    }
}

fn parse_time(value: &str, field: &'static str) -> Result<DateTime<Utc>, AppError> {
    DateTime::parse_from_rfc3339(value.trim())
        .map(|value| value.with_timezone(&Utc))
        .map_err(|error| invalid_field(field, error))
}

fn optional_time(value: &str, field: &'static str) -> Result<Option<DateTime<Utc>>, AppError> {
    if value.trim().is_empty() {
        Ok(None)
    } else {
        parse_time(value, field).map(Some)
    }
}

fn optional_text(value: String) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn invalid_field(field: &'static str, error: impl std::fmt::Display) -> AppError {
    AppError::internal(
        "enterprise_grpc_invalid_access_review",
        format!("Enterprise gRPC returned invalid {field}: {error}"),
    )
}
