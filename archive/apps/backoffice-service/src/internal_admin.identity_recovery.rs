use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, post},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use crate::app::AppState;
use crate::backoffice_authorization::{
    BackofficePermission, require_operator_permission_headers, require_operator_role_grant,
    require_permission, require_strong_confirmation,
};
use crate::backoffice_dual_control::{require_dual_control, second_approver_principal_id};
use crate::billing_admin_access::actor_principal_id;
use crate::error::AppError;

#[derive(Debug, Deserialize)]
struct RecoveryDecisionRequest {
    confirm_code: String,
    reason: String,
}

#[derive(Debug, Serialize)]
struct RecoveryReviewView {
    id: Uuid,
    principal_id: Uuid,
    tenant_id: Uuid,
    email: String,
    status: String,
    risk_score: f64,
    risk_factors: serde_json::Value,
    review_available_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct RecoveryDecisionResult {
    request_id: Uuid,
    status: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/admin/risk/recovery-reviews", get(list_reviews))
        .route(
            "/admin/risk/recovery-reviews/{requestId}/approve",
            post(approve_review),
        )
        .route(
            "/admin/risk/recovery-reviews/{requestId}/reject",
            post(reject_review),
        )
}

async fn list_reviews(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<RecoveryReviewView>>, AppError> {
    require_permission(&headers, BackofficePermission::RiskMutate)?;
    require_operator_role_grant(&state.db, &headers).await?;
    let rows = sqlx::query(
        r#"
        SELECT id, principal_id, tenant_id, email, status, risk_score, risk_factors,
               review_available_at, created_at
        FROM enterprise_password_recovery_requests
        WHERE status = 'pending'
        ORDER BY created_at ASC
        LIMIT 100
        "#,
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|row| RecoveryReviewView {
                id: row.get("id"),
                principal_id: row.get("principal_id"),
                tenant_id: row.get("tenant_id"),
                email: row.get("email"),
                status: row.get("status"),
                risk_score: row.get("risk_score"),
                risk_factors: row.get("risk_factors"),
                review_available_at: row.get("review_available_at"),
                created_at: row.get("created_at"),
            })
            .collect(),
    ))
}

async fn approve_review(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(request_id): Path<Uuid>,
    Json(request): Json<RecoveryDecisionRequest>,
) -> Result<Json<RecoveryDecisionResult>, AppError> {
    authorize_decision(
        &state.db,
        &headers,
        &request,
        "APPROVE IDENTITY RECOVERY",
        request_id,
    )
    .await?;
    decide_review(&state.db, &headers, request_id, request.reason, true)
        .await
        .map(Json)
}

async fn reject_review(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(request_id): Path<Uuid>,
    Json(request): Json<RecoveryDecisionRequest>,
) -> Result<Json<RecoveryDecisionResult>, AppError> {
    authorize_decision(
        &state.db,
        &headers,
        &request,
        "REJECT IDENTITY RECOVERY",
        request_id,
    )
    .await?;
    decide_review(&state.db, &headers, request_id, request.reason, false)
        .await
        .map(Json)
}

async fn authorize_decision(
    db: &sqlx::PgPool,
    headers: &HeaderMap,
    request: &RecoveryDecisionRequest,
    confirmation: &str,
    request_id: Uuid,
) -> Result<(), AppError> {
    require_operator_permission_headers(headers, BackofficePermission::RiskMutate)?;
    require_strong_confirmation(&request.confirm_code, confirmation, request_id)?;
    require_dual_control(headers)?;
    require_operator_role_grant(db, headers).await?;
    crate::risk_decision_center_validation::validate_reason(&request.reason)
}

async fn decide_review(
    db: &sqlx::PgPool,
    headers: &HeaderMap,
    request_id: Uuid,
    reason: String,
    approved: bool,
) -> Result<RecoveryDecisionResult, AppError> {
    let actor_id = actor_principal_id(headers)?;
    let second_approver_id = second_approver_principal_id(headers)?;
    let mut tx = db.begin().await?;
    let request = sqlx::query(
        r#"
        SELECT principal_id, tenant_id, status, review_available_at
        FROM enterprise_password_recovery_requests
        WHERE id = $1
        FOR UPDATE
        "#,
    )
    .bind(request_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        AppError::not_found(
            "identity_recovery_review_not_found",
            "Identity recovery review not found.",
        )
    })?;
    let principal_id: Uuid = request.get("principal_id");
    let tenant_id: Uuid = request.get("tenant_id");
    let status: String = request.get("status");
    let review_available_at: Option<DateTime<Utc>> = request.get("review_available_at");
    if status != "pending" {
        return Err(AppError::conflict(
            "identity_recovery_review_already_decided",
            "Identity recovery review has already been decided.",
        ));
    }
    if approved && review_available_at.is_none_or(|value| value > Utc::now()) {
        return Err(AppError::forbidden(
            "identity_recovery_review_cooldown",
            "Identity recovery cannot be approved before its review delay expires.",
        ));
    }
    if actor_id == principal_id || second_approver_id == principal_id {
        return Err(AppError::forbidden(
            "identity_recovery_self_approval_forbidden",
            "The recovering principal cannot approve their own recovery.",
        ));
    }

    let next_status = if approved { "approved" } else { "rejected" };
    if approved {
        sqlx::query(
            r#"
            UPDATE enterprise_password_recovery_requests
            SET status = 'approved', approved_by_principal_id = $2, approved_at = NOW(),
                secondary_approved_by_principal_id = $3, secondary_approved_at = NOW(),
                approval_expires_at = NOW() + INTERVAL '24 hours',
                review_reason = $4, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(request_id)
        .bind(actor_id)
        .bind(second_approver_id)
        .bind(&reason)
        .execute(&mut *tx)
        .await?;
    } else {
        sqlx::query(
            r#"
            UPDATE enterprise_password_recovery_requests
            SET status = 'rejected', rejected_by_principal_id = $2, rejected_at = NOW(),
                secondary_approved_by_principal_id = $3, secondary_approved_at = NOW(),
                review_reason = $4, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(request_id)
        .bind(actor_id)
        .bind(second_approver_id)
        .bind(&reason)
        .execute(&mut *tx)
        .await?;
    }
    sqlx::query(
        r#"
        INSERT INTO audit_events (
          tenant_id, actor_principal_id, action, target_type, target_id, metadata, event_hash
        )
        VALUES ($1, $2, $3, 'identity_recovery_request', $4, $5, gen_random_uuid()::text)
        "#,
    )
    .bind(tenant_id)
    .bind(actor_id)
    .bind(format!("identity.recovery.{next_status}"))
    .bind(request_id)
    .bind(serde_json::json!({
        "principal_id": principal_id,
        "second_approver_principal_id": second_approver_id,
        "reason": reason,
    }))
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(RecoveryDecisionResult {
        request_id,
        status: next_status.to_string(),
    })
}
