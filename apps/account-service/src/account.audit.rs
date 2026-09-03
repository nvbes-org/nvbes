use axum::http::HeaderMap;
use serde_json::Value;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

pub fn correlation_id(headers: &HeaderMap) -> Uuid {
    headers
        .get("x-correlation-id")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| Uuid::parse_str(value).ok())
        .unwrap_or_else(Uuid::new_v4)
}

pub struct AuditInput<'a> {
    pub principal_id: Uuid,
    pub actor_principal_id: Uuid,
    pub event_type: &'a str,
    pub resource_type: &'a str,
    pub resource_id: Option<Uuid>,
    pub correlation_id: Uuid,
    pub details: Value,
}

pub async fn record(
    tx: &mut Transaction<'_, Postgres>,
    input: AuditInput<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO account_audit_events (id, principal_id, actor_principal_id, event_type, resource_type, resource_id, correlation_id, details) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
    )
    .bind(Uuid::new_v4())
    .bind(input.principal_id)
    .bind(input.actor_principal_id)
    .bind(input.event_type)
    .bind(input.resource_type)
    .bind(input.resource_id)
    .bind(input.correlation_id)
    .bind(input.details)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn enqueue(
    tx: &mut Transaction<'_, Postgres>,
    event_type: &str,
    aggregate_id: Uuid,
    payload: Value,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO account_outbox (id, event_type, aggregate_id, payload) VALUES ($1,$2,$3,$4)",
    )
    .bind(Uuid::new_v4())
    .bind(event_type)
    .bind(aggregate_id)
    .bind(payload)
    .execute(&mut **tx)
    .await?;
    Ok(())
}
