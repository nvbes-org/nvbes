use axum::{Json, Router, extract::State, http::HeaderMap, routing::get};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::app::AppState;
use crate::billing_admin_access::actor_principal_id;
use crate::billing_admin_types::BackofficeAccess;
use crate::error::AppError;
use crate::grpc_pb::nvbes::billing::v1 as billing_pb;

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
    let actor_principal_id = actor_principal_id(&headers)?;
    Ok(Json(
        load_pending_approvals(&state.db, &state.billing_grpc_endpoint, actor_principal_id).await?,
    ))
}

async fn load_pending_approvals(
    db: &PgPool,
    billing_grpc_endpoint: &str,
    actor_principal_id: Uuid,
) -> Result<PendingApprovalsSnapshot, AppError> {
    let mut items = Vec::new();
    items.extend(load_pending_marketplace_apps(db).await?);
    items.extend(load_pending_recovery_requests(db).await?);
    items.extend(load_pending_kyc_reviews(billing_grpc_endpoint, actor_principal_id).await?);
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

async fn load_pending_kyc_reviews(
    billing_grpc_endpoint: &str,
    actor_principal_id: Uuid,
) -> Result<Vec<PendingApprovalItem>, AppError> {
    let snapshot = match crate::billing_grpc::get_admin_billing_platform_center(
        billing_grpc_endpoint,
        BackofficeAccess {
            tenant_id: Uuid::nil(),
            actor_principal_id,
        },
    )
    .await
    {
        Ok(snapshot) => snapshot,
        Err(error) if error.code == "billing_grpc_unavailable" => {
            tracing::warn!(
                %billing_grpc_endpoint,
                "billing gRPC unavailable while loading pending KYC approvals"
            );
            return Ok(Vec::new());
        }
        Err(error) => return Err(error),
    };

    snapshot
        .kyc_profiles
        .into_iter()
        .filter(|profile| profile.review_status == "pending")
        .take(8)
        .map(|profile| {
            let requested_at =
                parse_billing_datetime(&profile.created_at, "kyc profile created_at")?;
            let expires_at = parse_billing_datetime(&profile.updated_at, "kyc profile updated_at")?;
            let target_label = profile_label(&profile);
            Ok(PendingApprovalItem {
                id: profile.id.clone(),
                center: "Billing Platform",
                action: "Review KYC profile",
                severity: "high",
                status: "pending",
                tenant_id: empty_to_none(profile.tenant_id),
                tenant_name: empty_to_none(profile.tenant_name),
                workspace_id: None,
                principal_id: None,
                target_type: "kyc_profile",
                target_id: profile.id,
                target_label,
                requested_by: None,
                requested_at,
                expires_at: Some(expires_at),
                required_role: "finance_admin",
                audit_hint: "REVIEW KYC",
            })
        })
        .collect()
}

fn profile_label(profile: &billing_pb::KycProfile) -> String {
    empty_to_none(profile.company_name.clone()).unwrap_or_else(|| "KYC profile".to_string())
}

fn parse_billing_datetime(value: &str, field: &'static str) -> Result<DateTime<Utc>, AppError> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|_| AppError::internal("billing_grpc_decode", field))
}

fn empty_to_none(value: String) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}
