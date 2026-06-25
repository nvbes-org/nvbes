use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::post,
};
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

use crate::app::AppState;
use crate::backoffice_authorization::{
    BackofficePermission, require_operator_permission_headers, require_operator_role_grant,
    require_strong_confirmation, require_strong_confirmation_for_value,
};
use crate::backoffice_dual_control::require_dual_control;
use crate::billing_admin_access::authorize_backoffice;
use crate::entitlements_center_mutations::{
    EntitlementActionInput, EntitlementActionResult, insert_entitlement_action, publish_changes,
    validate_code, validate_reason,
};
use crate::error::AppError;

#[derive(Debug, Deserialize)]
struct FeatureActionRequest {
    confirm_code: String,
    feature_code: String,
    reason: String,
    #[serde(default)]
    value: Value,
}

#[derive(Debug, Deserialize)]
struct QuotaOverrideRequest {
    confirm_code: String,
    quota_code: String,
    included_quantity: i64,
    reason: String,
}

#[derive(Debug, Deserialize)]
struct PublishEntitlementChangesRequest {
    confirm_code: String,
    reason: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/workspaces/{workspaceId}/admin/entitlements/grants",
            post(grant_feature_route),
        )
        .route(
            "/workspaces/{workspaceId}/admin/entitlements/revocations",
            post(revoke_feature_route),
        )
        .route(
            "/workspaces/{workspaceId}/admin/entitlements/quota-overrides",
            post(override_quota_route),
        )
        .route(
            "/workspaces/{workspaceId}/admin/entitlements/publish",
            post(publish_entitlement_changes_route),
        )
}

async fn grant_feature_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<FeatureActionRequest>,
) -> Result<Json<EntitlementActionResult>, AppError> {
    require_entitlements_mutation_for_value(
        &state.db,
        &headers,
        &request.confirm_code,
        "GRANT FEATURE",
        &request.feature_code,
    )
    .await?;
    validate_code(&request.feature_code, "feature_code")?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        insert_entitlement_action(
            &state.db,
            access,
            workspace_id,
            EntitlementActionInput {
                action_kind: "grant_feature",
                feature_code: Some(request.feature_code),
                quota_code: None,
                quantity: None,
                reason: request.reason,
                metadata: request.value,
                audit_action: "entitlements.feature.granted",
                status: "queued",
                published_change_count: 0,
            },
        )
        .await?,
    ))
}

async fn revoke_feature_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<FeatureActionRequest>,
) -> Result<Json<EntitlementActionResult>, AppError> {
    require_entitlements_mutation_for_value(
        &state.db,
        &headers,
        &request.confirm_code,
        "REVOKE FEATURE",
        &request.feature_code,
    )
    .await?;
    validate_code(&request.feature_code, "feature_code")?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        insert_entitlement_action(
            &state.db,
            access,
            workspace_id,
            EntitlementActionInput {
                action_kind: "revoke_feature",
                feature_code: Some(request.feature_code),
                quota_code: None,
                quantity: None,
                reason: request.reason,
                metadata: request.value,
                audit_action: "entitlements.feature.revoked",
                status: "queued",
                published_change_count: 0,
            },
        )
        .await?,
    ))
}

async fn override_quota_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<QuotaOverrideRequest>,
) -> Result<Json<EntitlementActionResult>, AppError> {
    require_entitlements_mutation_for_value(
        &state.db,
        &headers,
        &request.confirm_code,
        "OVERRIDE QUOTA",
        &request.quota_code,
    )
    .await?;
    validate_code(&request.quota_code, "quota_code")?;
    if request.included_quantity < 0 {
        return Err(AppError::bad_request(
            "invalid_quota_override",
            "Quota override quantity must be zero or greater.",
        ));
    }
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        insert_entitlement_action(
            &state.db,
            access,
            workspace_id,
            EntitlementActionInput {
                action_kind: "override_quota",
                feature_code: None,
                quota_code: Some(request.quota_code),
                quantity: Some(request.included_quantity),
                reason: request.reason,
                metadata: Value::Object(Default::default()),
                audit_action: "entitlements.quota.overridden",
                status: "queued",
                published_change_count: 0,
            },
        )
        .await?,
    ))
}

async fn publish_entitlement_changes_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<PublishEntitlementChangesRequest>,
) -> Result<Json<EntitlementActionResult>, AppError> {
    require_entitlements_mutation(
        &state.db,
        &headers,
        &request.confirm_code,
        "PUBLISH ENTITLEMENTS",
        workspace_id,
    )
    .await?;
    validate_reason(&request.reason)?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    publish_changes(&state.db, access, workspace_id, request.reason)
        .await
        .map(Json)
}

async fn require_entitlements_mutation(
    db: &sqlx::PgPool,
    headers: &HeaderMap,
    confirm_code: &str,
    expected_code: &str,
    target_id: Uuid,
) -> Result<(), AppError> {
    require_operator_permission_headers(headers, BackofficePermission::EntitlementsMutate)?;
    require_strong_confirmation(confirm_code, expected_code, target_id)?;
    require_dual_control(headers)?;
    require_operator_role_grant(db, headers).await
}

async fn require_entitlements_mutation_for_value(
    db: &sqlx::PgPool,
    headers: &HeaderMap,
    confirm_code: &str,
    expected_code: &str,
    target_id: &str,
) -> Result<(), AppError> {
    require_operator_permission_headers(headers, BackofficePermission::EntitlementsMutate)?;
    require_strong_confirmation_for_value(confirm_code, expected_code, target_id)?;
    require_dual_control(headers)?;
    require_operator_role_grant(db, headers).await
}
