use axum::{
    Json,
    extract::{Path, State},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    app::BillingState,
    audit::record_audit_event,
    auth::OperatorAuth,
    error::{BillingError, BillingResult},
};

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ReconciliationItem {
    pub id: Uuid,
    pub source_event_id: Option<String>,
    pub account_id: Option<Uuid>,
    pub reason: String,
    pub details: Value,
    pub status: String,
    pub resolved_by: Option<String>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolution_notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ResolveReconciliationRequest {
    pub resolution_notes: String,
}

#[derive(Debug, Serialize)]
pub struct OperatorOverviewResponse {
    pub active_subscriptions: i64,
    pub open_checkouts: i64,
    pub pending_reconciliations: i64,
    pub pending_outbox: i64,
}

pub async fn record_reconciliation_item(
    db: &PgPool,
    source_event_id: Option<&str>,
    account_id: Option<Uuid>,
    reason: &str,
    details: &Value,
) -> BillingResult<Uuid> {
    let id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO billing_reconciliation_items (source_event_id, account_id, reason, details)
        VALUES ($1, $2, $3, $4)
        RETURNING id
        "#,
    )
    .bind(source_event_id)
    .bind(account_id)
    .bind(reason)
    .bind(details)
    .fetch_one(db)
    .await?;

    Ok(id)
}

pub async fn operator_overview_handler(
    State(state): State<BillingState>,
    _auth: OperatorAuth,
) -> BillingResult<Json<OperatorOverviewResponse>> {
    let active_subs: i64 =
        sqlx::query_scalar("SELECT count(*) FROM billing_subscriptions WHERE status = 'active'")
            .fetch_one(&state.db)
            .await?;

    let open_checkouts: i64 =
        sqlx::query_scalar("SELECT count(*) FROM billing_checkout_sessions WHERE status = 'open'")
            .fetch_one(&state.db)
            .await?;

    let pending_reconciliations: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM billing_reconciliation_items WHERE status = 'pending'",
    )
    .fetch_one(&state.db)
    .await?;

    let pending_outbox: i64 =
        sqlx::query_scalar("SELECT count(*) FROM billing_outbox WHERE published_at IS NULL")
            .fetch_one(&state.db)
            .await?;

    Ok(Json(OperatorOverviewResponse {
        active_subscriptions: active_subs,
        open_checkouts,
        pending_reconciliations,
        pending_outbox,
    }))
}

pub async fn operator_list_reconciliations_handler(
    State(state): State<BillingState>,
    _auth: OperatorAuth,
) -> BillingResult<Json<Vec<ReconciliationItem>>> {
    let items = sqlx::query_as::<_, ReconciliationItem>(
        r#"
        SELECT id, source_event_id, account_id, reason, details, status,
               resolved_by, resolved_at, resolution_notes, created_at
        FROM billing_reconciliation_items
        WHERE status = 'pending'
        ORDER BY created_at DESC
        LIMIT 50
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(items))
}

pub async fn operator_resolve_reconciliation_handler(
    State(state): State<BillingState>,
    _auth: OperatorAuth,
    Path(id): Path<Uuid>,
    Json(payload): Json<ResolveReconciliationRequest>,
) -> BillingResult<Json<ReconciliationItem>> {
    let item = sqlx::query_as::<_, ReconciliationItem>(
        r#"
        UPDATE billing_reconciliation_items
        SET status = 'resolved',
            resolved_by = 'operator',
            resolved_at = clock_timestamp(),
            resolution_notes = $2
        WHERE id = $1
        RETURNING id, source_event_id, account_id, reason, details, status,
                  resolved_by, resolved_at, resolution_notes, created_at
        "#,
    )
    .bind(id)
    .bind(&payload.resolution_notes)
    .fetch_optional(&state.db)
    .await?
    .ok_or(BillingError::NotFound)?;

    if let Some(acc_id) = item.account_id {
        record_audit_event(
            &state.db,
            acc_id,
            "operator",
            "reconciliation_resolved",
            &serde_json::json!({
                "reconciliation_id": id,
                "notes": payload.resolution_notes,
            }),
        )
        .await?;
    }

    Ok(Json(item))
}
