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
    BackofficePermission, require_confirmation, require_idempotency_key, require_permission,
    require_strong_confirmation,
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
    require_compliance_mutation(&headers, &request.confirm_code, "REVOKE CONSENT")?;
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
    require_compliance_authorization(&headers)?;
    require_strong_confirmation(&request.confirm_code, "REQUEST ERASURE", principal_id)?;
    require_dual_control(&headers)?;
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
    require_compliance_mutation(&headers, &request.confirm_code, "REVIEW SUPPRESSION")?;
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

fn require_compliance_mutation(
    headers: &HeaderMap,
    confirm_code: &str,
    expected_code: &str,
) -> Result<(), AppError> {
    require_compliance_authorization(headers)?;
    require_confirmation(confirm_code, expected_code)
}

fn require_compliance_authorization(headers: &HeaderMap) -> Result<(), AppError> {
    require_idempotency_key(headers)?;
    require_permission(headers, BackofficePermission::ComplianceMutate)
}
