use axum::{Json, Router, extract::Path, extract::State, http::HeaderMap, routing::get};
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

use crate::app::AppState;
use crate::billing_admin_access::authorize_backoffice;
use crate::error::AppError;

#[derive(Debug, Serialize)]
struct ProviderEventFailure {
    id: Uuid,
    provider: String,
    provider_event_id: String,
    event_type: String,
    status: String,
    signature_valid: bool,
    payload_summary: Value,
    received_at: DateTime<Utc>,
    processed_at: Option<DateTime<Utc>>,
}

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/workspaces/{workspaceId}/billing/admin/provider-events/failures",
        get(list_provider_event_failures_route),
    )
}

async fn list_provider_event_failures_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<Vec<ProviderEventFailure>>, AppError> {
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    let failures = crate::billing_grpc::list_admin_provider_event_failures(
        &state.billing_grpc_endpoint,
        access,
        workspace_id,
        50,
    )
    .await?;
    Ok(Json(provider_event_failures_from_grpc(failures)?))
}

fn provider_event_failures_from_grpc(
    value: crate::grpc_pb::nvbes::billing::v1::AdminProviderEventFailures,
) -> Result<Vec<ProviderEventFailure>, AppError> {
    value
        .failures
        .into_iter()
        .map(provider_event_failure_from_grpc)
        .collect()
}

fn provider_event_failure_from_grpc(
    value: crate::grpc_pb::nvbes::billing::v1::AdminProviderEventFailure,
) -> Result<ProviderEventFailure, AppError> {
    Ok(ProviderEventFailure {
        id: parse_uuid(&value.id, "provider event id")?,
        provider: value.provider,
        provider_event_id: value.provider_event_id,
        event_type: value.event_type,
        status: value.status,
        signature_valid: value.signature_valid,
        payload_summary: parse_json(&value.payload_summary_json)?,
        received_at: parse_datetime(&value.received_at, "provider event received_at")?,
        processed_at: parse_optional_datetime(&value.processed_at, "provider event processed_at")?,
    })
}

fn parse_uuid(value: &str, field: &'static str) -> Result<Uuid, AppError> {
    Uuid::parse_str(value).map_err(|_| AppError::internal("billing_grpc_decode", field))
}

fn parse_json(value: &str) -> Result<Value, AppError> {
    serde_json::from_str(value)
        .map_err(|_| AppError::internal("billing_grpc_decode", "payload_summary"))
}

fn parse_datetime(value: &str, field: &'static str) -> Result<DateTime<Utc>, AppError> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|_| AppError::internal("billing_grpc_decode", field))
}

fn parse_optional_datetime(
    value: &str,
    field: &'static str,
) -> Result<Option<DateTime<Utc>>, AppError> {
    if value.trim().is_empty() {
        Ok(None)
    } else {
        parse_datetime(value, field).map(Some)
    }
}
