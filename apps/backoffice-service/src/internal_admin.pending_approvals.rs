use axum::{Json, Router, extract::State, http::HeaderMap, routing::get};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{PgPool, Row};

use crate::app::AppState;
use crate::billing_admin_access::actor_principal_id;
use crate::error::AppError;

#[derive(Debug, Serialize)]
struct PendingApprovalsSnapshot {
    pending_count: usize,
    critical_count: usize,
    overdue_count: usize,
    items: Vec<PendingApprovalItem>,
}

#[derive(Debug, Serialize)]
struct PendingApprovalItem {
    id: String,
    center: &'static str,
    action: &'static str,
    severity: &'static str,
    status: &'static str,
    tenant_id: Option<String>,
    tenant_name: Option<String>,
    workspace_id: Option<String>,
    principal_id: Option<String>,
    target_type: &'static str,
    target_id: String,
    target_label: String,
    requested_by: Option<String>,
    requested_at: DateTime<Utc>,
    expires_at: Option<DateTime<Utc>>,
    required_role: &'static str,
    audit_hint: &'static str,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/admin/pending-approvals", get(pending_approvals_route))
}

async fn pending_approvals_route(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<PendingApprovalsSnapshot>, AppError> {
    let _actor_id = actor_principal_id(&headers)?;
    Ok(Json(load_pending_approvals(&state.db).await?))
}

async fn load_pending_approvals(db: &PgPool) -> Result<PendingApprovalsSnapshot, AppError> {
    let mut items = Vec::new();
    items.extend(load_pending_marketplace_apps(db).await?);
    items.extend(load_pending_recovery_requests(db).await?);
    items.extend(load_pending_kyc_reviews(db).await?);
    items.sort_by_key(|item| item.requested_at);
    items.truncate(24);

    let pending_count = items.len();
    let critical_count = items
        .iter()
        .filter(|item| item.severity == "critical")
        .count();
    let now = Utc::now();
    let overdue_count = items
        .iter()
        .filter(|item| item.expires_at.is_some_and(|expires_at| expires_at < now))
        .count();

    Ok(PendingApprovalsSnapshot {
        pending_count,
        critical_count,
        overdue_count,
        items,
    })
}

async fn load_pending_marketplace_apps(db: &PgPool) -> Result<Vec<PendingApprovalItem>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT ma.id::text AS id, ma.tenant_id::text AS tenant_id, t.name AS tenant_name,
          ma.id::text AS target_id, oc.name AS target_label, ma.created_at AS requested_at,
          ma.updated_at AS expires_at
        FROM developer_marketplace_apps ma
        JOIN tenants t ON t.id = ma.tenant_id
        JOIN oauth_clients oc ON oc.tenant_id = ma.tenant_id AND oc.client_id = ma.client_id
        WHERE ma.status::text = 'pending'
        ORDER BY ma.created_at ASC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| PendingApprovalItem {
            id: row.get("id"),
            center: "Developer",
            action: "Approve marketplace app",
            severity: "high",
            status: "pending",
            tenant_id: row.get("tenant_id"),
            tenant_name: row.get("tenant_name"),
            workspace_id: None,
            principal_id: None,
            target_type: "marketplace_app",
            target_id: row.get("target_id"),
            target_label: row.get("target_label"),
            requested_by: None,
            requested_at: row.get("requested_at"),
            expires_at: row.get("expires_at"),
            required_role: "developer_admin",
            audit_hint: "APPROVE MARKETPLACE APP",
        })
        .collect())
}

async fn load_pending_recovery_requests(db: &PgPool) -> Result<Vec<PendingApprovalItem>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT epr.id::text AS id, epr.tenant_id::text AS tenant_id, t.name AS tenant_name,
          epr.principal_id::text AS principal_id, epr.email AS target_label,
          epr.created_at AS requested_at, epr.available_at AS expires_at
        FROM enterprise_password_recovery_requests epr
        JOIN tenants t ON t.id = epr.tenant_id
        WHERE epr.status = 'pending'
        ORDER BY epr.created_at ASC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| PendingApprovalItem {
            id: row.get("id"),
            center: "Identity Governance",
            action: "Review recovery request",
            severity: "critical",
            status: "pending",
            tenant_id: row.get("tenant_id"),
            tenant_name: row.get("tenant_name"),
            workspace_id: None,
            principal_id: row.get("principal_id"),
            target_type: "recovery_request",
            target_id: row.get("id"),
            target_label: row.get("target_label"),
            requested_by: None,
            requested_at: row.get("requested_at"),
            expires_at: row.get("expires_at"),
            required_role: "security_admin",
            audit_hint: "CANCEL RECOVERY",
        })
        .collect())
}

async fn load_pending_kyc_reviews(db: &PgPool) -> Result<Vec<PendingApprovalItem>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT kyc.id::text AS id, kyc.tenant_id::text AS tenant_id, t.name AS tenant_name,
          COALESCE(kyc.company_name, 'KYC profile') AS target_label,
          kyc.created_at AS requested_at,
          kyc.updated_at AS expires_at
        FROM billing_kyc_profiles kyc
        JOIN tenants t ON t.id = kyc.tenant_id
        WHERE kyc.review_status = 'pending'
        ORDER BY kyc.created_at ASC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| PendingApprovalItem {
            id: row.get("id"),
            center: "Billing Platform",
            action: "Review KYC profile",
            severity: "high",
            status: "pending",
            tenant_id: row.get("tenant_id"),
            tenant_name: row.get("tenant_name"),
            workspace_id: None,
            principal_id: None,
            target_type: "kyc_profile",
            target_id: row.get("id"),
            target_label: row.get("target_label"),
            requested_by: None,
            requested_at: row.get("requested_at"),
            expires_at: row.get("expires_at"),
            required_role: "finance_admin",
            audit_hint: "REVIEW KYC",
        })
        .collect())
}
