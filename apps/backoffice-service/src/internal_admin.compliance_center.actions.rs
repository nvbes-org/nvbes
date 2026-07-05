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
    require_strong_confirmation, require_strong_confirmation_for_value,
};
use crate::backoffice_dual_control::require_dual_control;
use crate::billing_admin_access::authorize_backoffice;
use crate::compliance_center_mutations::{request_erasure, review_suppression, revoke_consent};
use crate::compliance_center_types::ComplianceActionResult;
use crate::error::AppError;

#[derive(Debug, Deserialize)]
struct ComplianceReasonRequest {
    confirm_code: String,
    reason: String,
}

#[derive(Debug, Deserialize)]
struct SuppressionReviewRequest {
    confirm_code: String,
    email: String,
    reason: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/workspaces/{workspaceId}/admin/compliance/consents/{consentId}/revoke",
            post(revoke_consent_route),
        )
        .route(
            "/workspaces/{workspaceId}/admin/compliance/principals/{principalId}/erasure-request",
            post(request_erasure_route),
        )
        .route(
            "/workspaces/{workspaceId}/admin/compliance/suppressions/review",
            post(review_suppression_route),
        )
}

async fn revoke_consent_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, consent_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<ComplianceReasonRequest>,
) -> Result<Json<ComplianceActionResult>, AppError> {
    require_compliance_mutation(
        &state.db,
        &headers,
        &request.confirm_code,
        "REVOKE CONSENT",
        consent_id,
    )
    .await?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        revoke_consent(&state.db, access, workspace_id, consent_id, request.reason).await?,
    ))
}

async fn request_erasure_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, principal_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<ComplianceReasonRequest>,
) -> Result<Json<ComplianceActionResult>, AppError> {
    require_compliance_mutation(
        &state.db,
        &headers,
        &request.confirm_code,
        "REQUEST ERASURE",
        principal_id,
    )
    .await?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        request_erasure(
            &state.db,
            access,
            workspace_id,
            principal_id,
            request.reason,
        )
        .await?,
    ))
}

async fn review_suppression_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<SuppressionReviewRequest>,
) -> Result<Json<ComplianceActionResult>, AppError> {
    require_compliance_mutation_for_value(
        &state.db,
        &headers,
        &request.confirm_code,
        "REVIEW SUPPRESSION",
        &request.email,
    )
    .await?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        review_suppression(
            &state.db,
            access,
            workspace_id,
            request.email,
            request.reason,
        )
        .await?,
    ))
}

async fn require_compliance_mutation(
    db: &sqlx::PgPool,
    headers: &HeaderMap,
    confirm_code: &str,
    expected_code: &str,
    target_id: Uuid,
) -> Result<(), AppError> {
    require_compliance_authorization(headers)?;
    require_strong_confirmation(confirm_code, expected_code, target_id)?;
    require_dual_control(headers)?;
    require_operator_role_grant(db, headers).await
}

async fn require_compliance_mutation_for_value(
    db: &sqlx::PgPool,
    headers: &HeaderMap,
    confirm_code: &str,
    expected_code: &str,
    target_id: &str,
) -> Result<(), AppError> {
    require_compliance_authorization(headers)?;
    require_strong_confirmation_for_value(confirm_code, expected_code, target_id)?;
    require_dual_control(headers)?;
    require_operator_role_grant(db, headers).await
}

fn require_compliance_authorization(headers: &HeaderMap) -> Result<(), AppError> {
    require_operator_permission_headers(headers, BackofficePermission::ComplianceMutate)
}
