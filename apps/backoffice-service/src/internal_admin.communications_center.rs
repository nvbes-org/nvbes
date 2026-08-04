use axum::{Json, Router, extract::State, http::HeaderMap, routing::get};
use chrono::{DateTime, TimeZone, Utc};
use serde::Serialize;

use crate::app::AppState;
use crate::billing_admin_access::actor_principal_id;
use crate::error::AppError;

#[derive(Debug, Serialize)]
struct CommunicationsCenterSnapshot {
    queued_message_count: i64,
    sent_message_count_24h: i64,
    delivered_message_count_24h: i64,
    failed_message_count_24h: i64,
    suppressed_email_count: i64,
    webhook_event_count_24h: i64,
    unprocessed_event_count: i64,
    status_distribution: Vec<EmailStatusDistribution>,
    business_type_distribution: Vec<BusinessTypeDistribution>,
    recent_failures: Vec<RecentEmailFailure>,
    recent_suppressions: Vec<RecentEmailSuppression>,
    recent_unprocessed_events: Vec<RecentEmailEvent>,
}

#[derive(Debug, Serialize)]
struct EmailStatusDistribution {
    status: String,
    message_count: i64,
}

#[derive(Debug, Serialize)]
struct BusinessTypeDistribution {
    business_type: String,
    message_count: i64,
    failure_count: i64,
}

#[derive(Debug, Serialize)]
struct RecentEmailFailure {
    id: uuid::Uuid,
    business_type: String,
    recipient_email: String,
    provider_email_id: Option<String>,
    status: String,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct RecentEmailSuppression {
    email: String,
    reason: String,
    suppressed_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct RecentEmailEvent {
    id: uuid::Uuid,
    provider_event_id: String,
    provider_email_id: Option<String>,
    email: String,
    event_type: String,
    occurred_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
}

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/admin/communications-center",
        get(communications_center_route),
    )
}

async fn communications_center_route(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<CommunicationsCenterSnapshot>, AppError> {
    let actor_id = actor_principal_id(&headers)?;
    Ok(Json(load_communications_center(&state, actor_id).await?))
}

async fn load_communications_center(
    state: &AppState,
    actor_id: uuid::Uuid,
) -> Result<CommunicationsCenterSnapshot, AppError> {
    let snapshot = state.email_operations.snapshot(actor_id).await?;
    Ok(CommunicationsCenterSnapshot {
        queued_message_count: snapshot.queued_message_count,
        sent_message_count_24h: snapshot.sent_message_count_24h,
        delivered_message_count_24h: snapshot.delivered_message_count_24h,
        failed_message_count_24h: snapshot.failed_message_count_24h,
        suppressed_email_count: snapshot.suppressed_email_count,
        webhook_event_count_24h: snapshot.webhook_event_count_24h,
        unprocessed_event_count: snapshot.unprocessed_event_count,
        status_distribution: snapshot
            .status_distribution
            .into_iter()
            .map(|item| EmailStatusDistribution {
                status: item.status,
                message_count: item.message_count,
            })
            .collect(),
        business_type_distribution: snapshot
            .business_type_distribution
            .into_iter()
            .map(|item| BusinessTypeDistribution {
                business_type: item.business_type,
                message_count: item.message_count,
                failure_count: item.failure_count,
            })
            .collect(),
        recent_failures: snapshot
            .recent_failures
            .into_iter()
            .map(|item| {
                Ok(RecentEmailFailure {
                    id: parse_uuid(&item.id)?,
                    business_type: item.business_type,
                    recipient_email: item.recipient_email,
                    provider_email_id: item.provider_email_id,
                    status: item.status,
                    updated_at: parse_timestamp(item.updated_at)?,
                })
            })
            .collect::<Result<_, AppError>>()?,
        recent_suppressions: snapshot
            .recent_suppressions
            .into_iter()
            .map(|item| {
                Ok(RecentEmailSuppression {
                    email: item.email,
                    reason: item.reason,
                    suppressed_at: parse_timestamp(item.suppressed_at)?,
                })
            })
            .collect::<Result<_, AppError>>()?,
        recent_unprocessed_events: snapshot
            .recent_unprocessed_events
            .into_iter()
            .map(|item| {
                let received_at = parse_timestamp(item.received_at)?;
                Ok(RecentEmailEvent {
                    id: parse_uuid(&item.id)?,
                    provider_event_id: item.provider_event_id,
                    provider_email_id: item.provider_email_id,
                    email: item.email,
                    event_type: item.event_type,
                    occurred_at: received_at,
                    created_at: received_at,
                })
            })
            .collect::<Result<_, AppError>>()?,
    })
}

fn parse_uuid(value: &str) -> Result<uuid::Uuid, AppError> {
    uuid::Uuid::parse_str(value).map_err(|_| {
        AppError::internal(
            "email_operation_protocol_invalid",
            "Email operations returned an invalid identifier.",
        )
    })
}

fn parse_timestamp(value: Option<prost_types::Timestamp>) -> Result<DateTime<Utc>, AppError> {
    let value = value.ok_or_else(|| {
        AppError::internal(
            "email_operation_protocol_invalid",
            "Email operations returned a missing timestamp.",
        )
    })?;
    Utc.timestamp_opt(value.seconds, value.nanos as u32)
        .single()
        .ok_or_else(|| {
            AppError::internal(
                "email_operation_protocol_invalid",
                "Email operations returned an invalid timestamp.",
            )
        })
}
