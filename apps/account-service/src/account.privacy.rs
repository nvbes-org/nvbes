use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::{Value, json};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    app::AccountState,
    audit::{self, AuditInput},
    auth::Principal,
    error::{AccountError, AccountResult},
    profile::ensure_profile,
};

#[derive(Debug, Serialize, FromRow)]
struct ExportRequest {
    #[sqlx(rename = "id")]
    export_id: Uuid,
    status: String,
    requested_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, FromRow)]
struct ExportStatus {
    #[sqlx(rename = "id")]
    export_id: Uuid,
    status: String,
    requested_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    expires_at: Option<DateTime<Utc>>,
    last_error: Option<String>,
    #[sqlx(skip)]
    participants: Vec<Participant>,
}

#[derive(Debug, Serialize)]
struct Participant {
    participant: &'static str,
    status: String,
    attempts: u8,
    completed_at: Option<DateTime<Utc>>,
    last_error: Option<String>,
}

#[derive(Debug, Serialize, FromRow)]
struct ClosureRequest {
    #[sqlx(rename = "id")]
    saga_id: Uuid,
    status: String,
    requested_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, FromRow)]
struct ClosureStatus {
    #[sqlx(rename = "id")]
    saga_id: Uuid,
    status: String,
    requested_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    last_error: Option<String>,
    #[sqlx(skip)]
    participants: Vec<Participant>,
}

pub fn router(state: AccountState) -> Router {
    Router::new()
        .route("/api/v1/privacy/exports", post(request_export))
        .route("/api/v1/privacy/exports/latest", get(latest_export))
        .route(
            "/api/v1/privacy/exports/{export_id}/document",
            get(download_export),
        )
        .route("/api/v1/closure", get(get_closure).post(request_closure))
        .route("/api/v1/closure/cancel", post(cancel_closure))
        .with_state(state)
}

async fn request_export(
    State(state): State<AccountState>,
    principal: Principal,
    headers: HeaderMap,
) -> AccountResult<(StatusCode, Json<ExportRequest>)> {
    principal.require("account:export")?;
    principal.require_step_up()?;
    ensure_profile(&state.db, principal.id).await?;
    let mut tx = state.db.begin().await?;
    let existing = sqlx::query_as::<_, ExportRequest>("SELECT id,status,requested_at FROM account_exports WHERE principal_id=$1 AND status IN ('pending','processing') ORDER BY requested_at DESC LIMIT 1").bind(principal.id).fetch_optional(&mut *tx).await?;
    if let Some(value) = existing {
        tx.commit().await?;
        return Ok((StatusCode::ACCEPTED, Json(value)));
    }
    let id = Uuid::new_v4();
    let value = sqlx::query_as("INSERT INTO account_exports(id,principal_id,status) VALUES($1,$2,'pending') RETURNING id,status,requested_at").bind(id).bind(principal.id).fetch_one(&mut *tx).await?;
    audit::record(
        &mut tx,
        AuditInput {
            principal_id: principal.id,
            actor_principal_id: principal.id,
            event_type: "account.export.requested",
            resource_type: "export",
            resource_id: Some(id),
            correlation_id: audit::correlation_id(&headers),
            details: json!({}),
        },
    )
    .await?;
    audit::enqueue(
        &mut tx,
        "account.export.requested.v1",
        id,
        json!({"export_id": id, "principal_id": principal.id}),
    )
    .await?;
    tx.commit().await?;
    Ok((StatusCode::ACCEPTED, Json(value)))
}

async fn latest_export(
    State(state): State<AccountState>,
    principal: Principal,
) -> AccountResult<Json<ExportStatus>> {
    principal.require("account:export")?;
    let row = sqlx::query_as::<_, ExportStatus>("SELECT id,status,requested_at,updated_at,completed_at,expires_at,last_error FROM account_exports WHERE principal_id=$1 ORDER BY requested_at DESC LIMIT 1").bind(principal.id).fetch_optional(&state.db).await?.ok_or(AccountError::NotFound)?;
    Ok(Json(with_participant(row)))
}

async fn download_export(
    State(state): State<AccountState>,
    principal: Principal,
    axum::extract::Path(export_id): axum::extract::Path<Uuid>,
) -> AccountResult<Json<Value>> {
    principal.require("account:export")?;
    principal.require_step_up()?;
    sqlx::query_scalar("SELECT document FROM account_exports WHERE id=$1 AND principal_id=$2 AND status='completed' AND expires_at>clock_timestamp()")
        .bind(export_id).bind(principal.id).fetch_optional(&state.db).await?.map(Json).ok_or(AccountError::NotFound)
}

async fn request_closure(
    State(state): State<AccountState>,
    principal: Principal,
    headers: HeaderMap,
) -> AccountResult<(StatusCode, Json<ClosureRequest>)> {
    principal.require("account:close")?;
    principal.require_step_up()?;
    ensure_profile(&state.db, principal.id).await?;
    let mut tx = state.db.begin().await?;
    if let Some(value) = sqlx::query_as::<_, ClosureRequest>("SELECT id,status,requested_at FROM account_closures WHERE principal_id=$1 AND status IN ('pending','processing') LIMIT 1").bind(principal.id).fetch_optional(&mut *tx).await? { tx.commit().await?; return Ok((StatusCode::ACCEPTED, Json(value))); }
    let id = Uuid::new_v4();
    let value = sqlx::query_as("INSERT INTO account_closures(id,principal_id,status,execute_after) VALUES($1,$2,'pending',clock_timestamp()+interval '7 days') RETURNING id,status,requested_at").bind(id).bind(principal.id).fetch_one(&mut *tx).await?;
    sqlx::query("UPDATE account_profiles SET lifecycle_status='closure_pending',updated_at=clock_timestamp() WHERE principal_id=$1 AND lifecycle_status='active'").bind(principal.id).execute(&mut *tx).await?;
    audit::record(
        &mut tx,
        AuditInput {
            principal_id: principal.id,
            actor_principal_id: principal.id,
            event_type: "account.closure.requested",
            resource_type: "closure",
            resource_id: Some(id),
            correlation_id: audit::correlation_id(&headers),
            details: json!({"grace_period_days": 7}),
        },
    )
    .await?;
    audit::enqueue(
        &mut tx,
        "account.closure.requested.v1",
        id,
        json!({"saga_id": id, "principal_id": principal.id}),
    )
    .await?;
    tx.commit().await?;
    Ok((StatusCode::ACCEPTED, Json(value)))
}

async fn get_closure(
    State(state): State<AccountState>,
    principal: Principal,
) -> AccountResult<Json<ClosureStatus>> {
    principal.require("account:close")?;
    let row = sqlx::query_as::<_, ClosureStatus>("SELECT id,status,requested_at,updated_at,completed_at,last_error FROM account_closures WHERE principal_id=$1 ORDER BY requested_at DESC LIMIT 1").bind(principal.id).fetch_optional(&state.db).await?.ok_or(AccountError::NotFound)?;
    Ok(Json(with_closure_participant(row)))
}

async fn cancel_closure(
    State(state): State<AccountState>,
    principal: Principal,
    headers: HeaderMap,
) -> AccountResult<StatusCode> {
    principal.require("account:close")?;
    principal.require_step_up()?;
    let mut tx = state.db.begin().await?;
    let id: Uuid = sqlx::query_scalar("UPDATE account_closures SET status='cancelled',updated_at=clock_timestamp() WHERE principal_id=$1 AND status='pending' RETURNING id").bind(principal.id).fetch_optional(&mut *tx).await?.ok_or(AccountError::Conflict)?;
    sqlx::query("UPDATE account_profiles SET lifecycle_status='active',updated_at=clock_timestamp() WHERE principal_id=$1 AND lifecycle_status='closure_pending'").bind(principal.id).execute(&mut *tx).await?;
    audit::record(
        &mut tx,
        AuditInput {
            principal_id: principal.id,
            actor_principal_id: principal.id,
            event_type: "account.closure.cancelled",
            resource_type: "closure",
            resource_id: Some(id),
            correlation_id: audit::correlation_id(&headers),
            details: json!({}),
        },
    )
    .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

fn with_participant(mut row: ExportStatus) -> ExportStatus {
    row.participants = vec![Participant {
        participant: "account",
        status: row.status.clone(),
        attempts: 1,
        completed_at: row.completed_at,
        last_error: row.last_error.clone(),
    }];
    row
}

fn with_closure_participant(mut row: ClosureStatus) -> ClosureStatus {
    row.participants = vec![Participant {
        participant: "account",
        status: row.status.clone(),
        attempts: 1,
        completed_at: row.completed_at,
        last_error: row.last_error.clone(),
    }];
    row
}
