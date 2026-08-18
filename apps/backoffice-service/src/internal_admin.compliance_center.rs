use axum::{Json, Router, extract::State, http::HeaderMap, routing::get};
use chrono::{DateTime, TimeZone, Utc};
use serde::Serialize;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::app::AppState;
use crate::billing_admin_access::actor_principal_id;
use crate::error::AppError;

#[derive(Debug, Serialize)]
struct ComplianceCenterSnapshot {
    active_consent_count: i64,
    revoked_consent_count_30d: i64,
    suppressed_email_count: i64,
    email_bounce_count_24h: i64,
    email_delivery_failure_count_24h: i64,
    unverified_user_count: i64,
    recent_revoked_consents: Vec<RecentRevokedConsent>,
    recent_suppressed_emails: Vec<RecentSuppressedEmail>,
}

#[derive(Debug, Serialize)]
struct RecentRevokedConsent {
    id: Uuid,
    principal_id: Uuid,
    email: Option<String>,
    tenant_id: Uuid,
    tenant_name: String,
    consent_type: String,
    document_version: String,
    revoked_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct RecentSuppressedEmail {
    email: String,
    reason: String,
    principal_id: Option<Uuid>,
    tenant_id: Option<Uuid>,
    tenant_name: Option<String>,
    suppressed_at: DateTime<Utc>,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/admin/compliance-center", get(compliance_center_route))
}

async fn compliance_center_route(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ComplianceCenterSnapshot>, AppError> {
    let actor_id = actor_principal_id(&headers)?;
    Ok(Json(load_compliance_center(&state, actor_id).await?))
}

async fn load_compliance_center(
    state: &AppState,
    actor_id: Uuid,
) -> Result<ComplianceCenterSnapshot, AppError> {
    let email = state.email_operations.snapshot(actor_id).await?;
    let metrics = sqlx::query(
        r#"
        SELECT
          (
            SELECT COUNT(*) FROM user_consents
            WHERE revoked_at IS NULL
          ) AS active_consent_count,
          (
            SELECT COUNT(*) FROM user_consents
            WHERE revoked_at >= NOW() - INTERVAL '30 days'
          ) AS revoked_consent_count_30d,
          (
            SELECT COUNT(*) FROM users
            WHERE email_verified_at IS NULL AND status::text <> 'deleted'
          ) AS unverified_user_count
        "#,
    )
    .fetch_one(&state.db)
    .await?;

    Ok(ComplianceCenterSnapshot {
        active_consent_count: metrics.get("active_consent_count"),
        revoked_consent_count_30d: metrics.get("revoked_consent_count_30d"),
        suppressed_email_count: email.suppressed_email_count,
        email_bounce_count_24h: email.bounce_event_count_24h,
        email_delivery_failure_count_24h: email.failed_message_count_24h,
        unverified_user_count: metrics.get("unverified_user_count"),
        recent_revoked_consents: load_recent_revoked_consents(&state.db).await?,
        recent_suppressed_emails: load_recent_suppressed_emails(
            &state.db,
            email.recent_suppressions,
        )
        .await?,
    })
}

async fn load_recent_revoked_consents(db: &PgPool) -> Result<Vec<RecentRevokedConsent>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT uc.id, uc.principal_id, u.email, p.tenant_id, t.name AS tenant_name,
          uc.consent_type, uc.document_version, uc.revoked_at
        FROM user_consents uc
        JOIN principals p ON p.id = uc.principal_id
        JOIN tenants t ON t.id = p.tenant_id
        LEFT JOIN users u ON u.principal_id = uc.principal_id
        WHERE uc.revoked_at IS NOT NULL
        ORDER BY uc.revoked_at DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| RecentRevokedConsent {
            id: row.get("id"),
            principal_id: row.get("principal_id"),
            email: row.get("email"),
            tenant_id: row.get("tenant_id"),
            tenant_name: row.get("tenant_name"),
            consent_type: row.get("consent_type"),
            document_version: row.get("document_version"),
            revoked_at: row.get("revoked_at"),
        })
        .collect())
}

async fn load_recent_suppressed_emails(
    db: &PgPool,
    suppressions: Vec<nvbes_email::proto::nvbes::email::v1::EmailSuppressionSummary>,
) -> Result<Vec<RecentSuppressedEmail>, AppError> {
    let emails = suppressions
        .iter()
        .map(|item| item.email.to_ascii_lowercase())
        .collect::<Vec<_>>();
    let rows = sqlx::query(
        r#"
        SELECT lower(u.email) AS email, u.principal_id, p.tenant_id, t.name AS tenant_name
        FROM users u
        LEFT JOIN principals p ON p.id = u.principal_id
        LEFT JOIN tenants t ON t.id = p.tenant_id
        WHERE lower(u.email) = ANY($1)
        "#,
    )
    .bind(&emails)
    .fetch_all(db)
    .await?;
    let identities: std::collections::HashMap<
        String,
        (Option<Uuid>, Option<Uuid>, Option<String>),
    > = rows
        .into_iter()
        .map(|row| {
            (
                row.get::<String, _>("email"),
                (
                    row.get("principal_id"),
                    row.get("tenant_id"),
                    row.get("tenant_name"),
                ),
            )
        })
        .collect::<std::collections::HashMap<_, _>>();

    suppressions
        .into_iter()
        .map(|item| {
            let identity = identities.get(&item.email.to_ascii_lowercase());
            Ok(RecentSuppressedEmail {
                email: item.email,
                reason: item.reason,
                principal_id: identity.and_then(|value| value.0),
                tenant_id: identity.and_then(|value| value.1),
                tenant_name: identity.and_then(|value| value.2.clone()),
                suppressed_at: parse_timestamp(item.suppressed_at)?,
            })
        })
        .collect()
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
