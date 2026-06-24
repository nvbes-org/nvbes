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
use crate::error::AppError;
use crate::revenue_center_mutations::{
    close_dunning_case, hold_invoice, release_invoice, reopen_dunning_case, resolve_dispute,
    review_dispute,
};
use crate::revenue_center_types::RevenueActionResult;

#[derive(Debug, Deserialize)]
struct RevenueActionRequest {
    confirm_code: String,
    reason: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/workspaces/{workspaceId}/admin/revenue/dunning-cases/{caseId}/close",
            post(close_dunning_case_route),
        )
        .route(
            "/workspaces/{workspaceId}/admin/revenue/dunning-cases/{caseId}/reopen",
            post(reopen_dunning_case_route),
        )
        .route(
            "/workspaces/{workspaceId}/admin/revenue/invoices/{invoiceId}/hold",
            post(hold_invoice_route),
        )
        .route(
            "/workspaces/{workspaceId}/admin/revenue/invoices/{invoiceId}/release",
            post(release_invoice_route),
        )
        .route(
            "/workspaces/{workspaceId}/admin/revenue/disputes/{disputeId}/review",
            post(review_dispute_route),
        )
        .route(
            "/workspaces/{workspaceId}/admin/revenue/disputes/{disputeId}/resolve",
            post(resolve_dispute_route),
        )
}

async fn close_dunning_case_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, case_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<RevenueActionRequest>,
) -> Result<Json<RevenueActionResult>, AppError> {
    require_revenue_mutation(&headers, &request.confirm_code, "CLOSE DUNNING CASE")?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        close_dunning_case(&state.db, access, workspace_id, case_id, request.reason).await?,
    ))
}

async fn reopen_dunning_case_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, case_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<RevenueActionRequest>,
) -> Result<Json<RevenueActionResult>, AppError> {
    require_revenue_mutation(&headers, &request.confirm_code, "REOPEN DUNNING CASE")?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        reopen_dunning_case(&state.db, access, workspace_id, case_id, request.reason).await?,
    ))
}

async fn hold_invoice_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, invoice_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<RevenueActionRequest>,
) -> Result<Json<RevenueActionResult>, AppError> {
    require_revenue_authorization(&headers)?;
    require_strong_confirmation(&request.confirm_code, "HOLD INVOICE", invoice_id)?;
    require_dual_control(&headers)?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        hold_invoice(&state.db, access, workspace_id, invoice_id, request.reason).await?,
    ))
}

async fn release_invoice_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, invoice_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<RevenueActionRequest>,
) -> Result<Json<RevenueActionResult>, AppError> {
    require_revenue_mutation(&headers, &request.confirm_code, "RELEASE INVOICE")?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        release_invoice(&state.db, access, workspace_id, invoice_id, request.reason).await?,
    ))
}

async fn review_dispute_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, dispute_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<RevenueActionRequest>,
) -> Result<Json<RevenueActionResult>, AppError> {
    require_revenue_mutation(&headers, &request.confirm_code, "REVIEW DISPUTE")?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        review_dispute(&state.db, access, workspace_id, dispute_id, request.reason).await?,
    ))
}

async fn resolve_dispute_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, dispute_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<RevenueActionRequest>,
) -> Result<Json<RevenueActionResult>, AppError> {
    require_revenue_mutation(&headers, &request.confirm_code, "RESOLVE DISPUTE")?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        resolve_dispute(&state.db, access, workspace_id, dispute_id, request.reason).await?,
    ))
}

fn require_revenue_mutation(
    headers: &HeaderMap,
    confirm_code: &str,
    expected_code: &str,
) -> Result<(), AppError> {
    require_revenue_authorization(headers)?;
    require_confirmation(confirm_code, expected_code)
}

fn require_revenue_authorization(headers: &HeaderMap) -> Result<(), AppError> {
    require_idempotency_key(headers)?;
    require_permission(headers, BackofficePermission::RevenueMutate)
}
