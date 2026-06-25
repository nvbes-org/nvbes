use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::post,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::app::AppState;
use crate::backoffice_authorization::{
    BackofficePermission, require_operator_permission_headers, require_operator_role_grant,
    require_strong_confirmation,
};
use crate::backoffice_dual_control::require_dual_control;
use crate::billing_admin_access::authorize_backoffice;
use crate::billing_platform_center_mutations::{
    activate_einvoicing_profile, approve_kyc_profile, disable_routing_rule, enable_routing_rule,
    reject_kyc_profile,
};
use crate::billing_platform_center_types::BillingPlatformActionResult;
use crate::error::AppError;

#[derive(Debug, Deserialize)]
struct BillingPlatformActionRequest {
    confirm_code: String,
    reason: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/workspaces/{workspaceId}/admin/billing-platform/routing-rules/{ruleId}/enable",
            post(enable_routing_rule_route),
        )
        .route(
            "/workspaces/{workspaceId}/admin/billing-platform/routing-rules/{ruleId}/disable",
            post(disable_routing_rule_route),
        )
        .route(
            "/workspaces/{workspaceId}/admin/billing-platform/kyc-profiles/{profileId}/approve",
            post(approve_kyc_profile_route),
        )
        .route(
            "/workspaces/{workspaceId}/admin/billing-platform/kyc-profiles/{profileId}/reject",
            post(reject_kyc_profile_route),
        )
        .route(
            "/workspaces/{workspaceId}/admin/billing-platform/einvoicing-profiles/{profileId}/activate",
            post(activate_einvoicing_profile_route),
        )
}

async fn enable_routing_rule_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, rule_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<BillingPlatformActionRequest>,
) -> Result<Json<BillingPlatformActionResult>, AppError> {
    require_billing_platform_mutation(
        &state.db,
        &headers,
        &request.confirm_code,
        "ENABLE ROUTING RULE",
        rule_id,
    )
    .await?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        enable_routing_rule(&state.db, access, workspace_id, rule_id, request.reason).await?,
    ))
}

async fn disable_routing_rule_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, rule_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<BillingPlatformActionRequest>,
) -> Result<Json<BillingPlatformActionResult>, AppError> {
    require_billing_platform_mutation(
        &state.db,
        &headers,
        &request.confirm_code,
        "DISABLE ROUTING RULE",
        rule_id,
    )
    .await?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        disable_routing_rule(&state.db, access, workspace_id, rule_id, request.reason).await?,
    ))
}

async fn approve_kyc_profile_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, profile_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<BillingPlatformActionRequest>,
) -> Result<Json<BillingPlatformActionResult>, AppError> {
    require_billing_platform_mutation(
        &state.db,
        &headers,
        &request.confirm_code,
        "APPROVE KYC",
        profile_id,
    )
    .await?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        approve_kyc_profile(&state.db, access, workspace_id, profile_id, request.reason).await?,
    ))
}

async fn reject_kyc_profile_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, profile_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<BillingPlatformActionRequest>,
) -> Result<Json<BillingPlatformActionResult>, AppError> {
    require_billing_platform_mutation(
        &state.db,
        &headers,
        &request.confirm_code,
        "REJECT KYC",
        profile_id,
    )
    .await?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        reject_kyc_profile(&state.db, access, workspace_id, profile_id, request.reason).await?,
    ))
}

async fn activate_einvoicing_profile_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, profile_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<BillingPlatformActionRequest>,
) -> Result<Json<BillingPlatformActionResult>, AppError> {
    require_billing_platform_mutation(
        &state.db,
        &headers,
        &request.confirm_code,
        "ACTIVATE EINVOICING",
        profile_id,
    )
    .await?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        activate_einvoicing_profile(&state.db, access, workspace_id, profile_id, request.reason)
            .await?,
    ))
}

async fn require_billing_platform_mutation(
    db: &sqlx::PgPool,
    headers: &HeaderMap,
    confirm_code: &str,
    expected_code: &str,
    target_id: Uuid,
) -> Result<(), AppError> {
    require_billing_platform_authorization(headers)?;
    require_strong_confirmation(confirm_code, expected_code, target_id)?;
    require_dual_control(headers)?;
    require_operator_role_grant(db, headers).await
}

fn require_billing_platform_authorization(headers: &HeaderMap) -> Result<(), AppError> {
    require_operator_permission_headers(headers, BackofficePermission::BillingPlatformMutate)
}
