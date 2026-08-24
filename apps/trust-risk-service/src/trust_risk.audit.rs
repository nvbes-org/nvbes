use chrono::{DateTime, Duration, Utc};
use sqlx::{Postgres, Transaction};

pub struct AuditEvent<'a> {
    pub action: &'a str,
    pub actor: &'a str,
    pub reason: &'a str,
    pub target_type: &'a str,
    pub target_id: &'a str,
    pub detail: serde_json::Value,
}

pub async fn append(
    tx: &mut Transaction<'_, Postgres>,
    event: AuditEvent<'_>,
    retention_days: u32,
) -> Result<DateTime<Utc>, sqlx::Error> {
    let expires_at = Utc::now() + Duration::days(i64::from(retention_days));
    sqlx::query_scalar(
        r#"
        INSERT INTO trust_risk_audit_events (
            action, actor, reason, target_type, target_id, detail, expires_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING occurred_at
        "#,
    )
    .bind(event.action)
    .bind(event.actor)
    .bind(event.reason)
    .bind(event.target_type)
    .bind(event.target_id)
    .bind(event.detail)
    .bind(expires_at)
    .fetch_one(&mut **tx)
    .await
}
