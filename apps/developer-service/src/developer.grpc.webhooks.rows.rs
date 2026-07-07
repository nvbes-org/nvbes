use chrono::{DateTime, Utc};
use sqlx::{Row, postgres::PgRow};
use uuid::Uuid;

use crate::grpc::pb::nvbes::developer::v1 as developer;

pub fn endpoint_from_row(row: PgRow, signing_secret: String) -> developer::WebhookEndpoint {
    let events = row.get::<Vec<String>, _>("event_types");
    endpoint_with_events(row, events, signing_secret)
}

pub fn endpoint_with_events(
    row: PgRow,
    events: Vec<String>,
    signing_secret: String,
) -> developer::WebhookEndpoint {
    developer::WebhookEndpoint {
        endpoint_id: row.get::<Uuid, _>("id").to_string(),
        tenant_id: row.get::<Uuid, _>("tenant_id").to_string(),
        name: row.get("name"),
        url: row.get("url"),
        event_types: events,
        status: row.get("status"),
        signing_secret,
        signing_secret_last4: row.get("signing_secret_last4"),
        created_at: time_string(row.get("created_at")),
        updated_at: time_string(row.get("updated_at")),
        failed_delivery_count: row.get::<i64, _>("failed_delivery_count"),
    }
}

pub fn delivery_from_row(row: PgRow) -> developer::WebhookDelivery {
    let response_status = row
        .get::<Option<i32>, _>("response_status")
        .map(|status| status.to_string())
        .unwrap_or_default();
    let delivered_at = row
        .get::<Option<DateTime<Utc>>, _>("delivered_at")
        .map(time_string)
        .unwrap_or_default();
    let replayed_from_delivery_id = row
        .get::<Option<Uuid>, _>("replayed_from_delivery_id")
        .map(|id| id.to_string())
        .unwrap_or_default();

    developer::WebhookDelivery {
        delivery_id: row.get::<Uuid, _>("id").to_string(),
        endpoint_id: row.get::<Uuid, _>("endpoint_id").to_string(),
        event_type: row.get("event_type"),
        status: row.get("status"),
        attempted_at: time_string(row.get("created_at")),
        event_id: row.get::<Uuid, _>("event_id").to_string(),
        attempt_count: row.get("attempt_count"),
        response_status,
        error_message: row
            .get::<Option<String>, _>("error_message")
            .unwrap_or_default(),
        created_at: time_string(row.get("created_at")),
        delivered_at,
        replayed_from_delivery_id,
    }
}

pub fn api_log_from_row(row: PgRow) -> developer::ApiLogEntry {
    let id = row.get::<Uuid, _>("id").to_string();
    let created_at = time_string(row.get("created_at"));
    developer::ApiLogEntry {
        request_id: id.clone(),
        client_id: String::new(),
        method: String::new(),
        path: String::new(),
        status_code: 0,
        occurred_at: created_at.clone(),
        id,
        source: "webhook".to_string(),
        event_type: row.get("event_type"),
        severity: row.get("severity"),
        message: row.get("message"),
        created_at,
    }
}

fn time_string(value: DateTime<Utc>) -> String {
    value.to_rfc3339()
}
