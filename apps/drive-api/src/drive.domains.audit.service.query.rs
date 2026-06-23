use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::{
    domains::audit::service::events_to_csv, domains::authz::WorkspaceAccess, http::error::AppError,
};

use super::{
    AuditEventView, AuditEventsResponse, AuditExportResponse, AuditRecordInput, EXPORT_LIMIT,
    ListAuditEventsInput, normalize_limit, normalize_optional_text,
};

pub async fn list_events(
    db: &PgPool,
    access: &WorkspaceAccess,
    input: ListAuditEventsInput,
) -> Result<AuditEventsResponse, AppError> {
    let limit = normalize_limit(input.limit);
    let rows = fetch_events(db, access.workspace_id, input, limit).await?;
    let next_before = if rows.len() == limit as usize {
        rows.last().map(|event| event.id)
    } else {
        None
    };

    Ok(AuditEventsResponse {
        workspace_id: access.workspace_id,
        events: rows,
        next_before,
    })
}

pub async fn export_events(
    db: &PgPool,
    access: &WorkspaceAccess,
) -> Result<AuditExportResponse, AppError> {
    let rows = fetch_events(
        db,
        access.workspace_id,
        ListAuditEventsInput {
            limit: Some(EXPORT_LIMIT),
            before_id: None,
            action: None,
            actor_user_id: None,
            actor_principal_id: None,
            geo_network_kind: None,
            min_geo_risk_score: None,
            geo_risk_label: None,
            network_block_reason: None,
        },
        EXPORT_LIMIT,
    )
    .await?;

    Ok(AuditExportResponse {
        workspace_id: access.workspace_id,
        filename: format!("nvbes-audit-{}.csv", access.workspace_id),
        content_type: "text/csv; charset=utf-8",
        body: events_to_csv(&rows),
    })
}

pub async fn record_event(db: &PgPool, input: AuditRecordInput<'_>) -> Result<(), AppError> {
    let mut tx = db.begin().await?;
    let metadata = crate::domains::audit::geo::enrich_audit_metadata_tx(
        &mut tx,
        input.workspace_id,
        input.ip,
        input.action,
        input.metadata,
    )
    .await?;

    sqlx::query(
        r#"
        INSERT INTO audit_events (
          workspace_id,
          actor_user_id,
          actor_principal_id,
          action,
          target_type,
          target_id,
          ip,
          user_agent,
          metadata
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7::inet, $8, $9)
        "#,
    )
    .bind(input.workspace_id)
    .bind(input.actor_user_id)
    .bind(input.actor_principal_id)
    .bind(input.action)
    .bind(input.target_type)
    .bind(input.target_id)
    .bind(input.ip)
    .bind(input.user_agent)
    .bind(sqlx::types::Json(metadata))
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(())
}

async fn fetch_events(
    db: &PgPool,
    workspace_id: Uuid,
    input: ListAuditEventsInput,
    limit: i64,
) -> Result<Vec<AuditEventView>, AppError> {
    let before_created_at = if let Some(before_id) = input.before_id {
        sqlx::query_scalar::<_, DateTime<Utc>>(
            "SELECT created_at FROM audit_events WHERE id = $1 AND workspace_id = $2",
        )
        .bind(before_id)
        .bind(workspace_id)
        .fetch_optional(db)
        .await?
    } else {
        None
    };

    let rows = sqlx::query(
        r#"
        SELECT
          ae.id,
          ae.workspace_id,
          ae.actor_user_id,
          ae.actor_principal_id,
          u.email AS actor_email,
          ae.action,
          ae.target_type,
          ae.target_id,
          ae.ip::text AS ip,
          ae.user_agent,
          ae.metadata #>> '{geo,geo_country_code}' AS geo_country_code,
          ae.metadata #>> '{geo,geo_network_kind}' AS geo_network_kind,
          NULLIF(ae.metadata #>> '{geo,geo_risk_score}', '')::bigint AS geo_risk_score,
          COALESCE(ae.metadata #> '{geo,geo_risk_labels}', '[]'::jsonb) AS geo_risk_labels,
          ae.metadata->>'network_block_reason' AS network_block_reason,
          ae.metadata,
          ae.previous_event_hash,
          ae.event_hash,
          ae.created_at
        FROM audit_events ae
        LEFT JOIN users u ON u.id = ae.actor_user_id
        WHERE ae.workspace_id = $1
          AND ($2::timestamptz IS NULL OR ae.created_at < $2)
          AND ($3::text IS NULL OR ae.action = $3)
          AND ($4::uuid IS NULL OR ae.actor_user_id = $4)
          AND ($5::uuid IS NULL OR ae.actor_principal_id = $5)
          AND ($6::text IS NULL OR ae.metadata #>> '{geo,geo_network_kind}' = $6)
          AND ($7::bigint IS NULL OR COALESCE(NULLIF(ae.metadata #>> '{geo,geo_risk_score}', '')::bigint, 0) >= $7)
          AND (
            $8::text IS NULL OR EXISTS (
              SELECT 1
              FROM jsonb_array_elements_text(COALESCE(ae.metadata #> '{geo,geo_risk_labels}', '[]'::jsonb)) AS label(value)
              WHERE lower(label.value) = $8
            )
          )
          AND ($9::text IS NULL OR ae.metadata->>'network_block_reason' = $9)
        ORDER BY ae.created_at DESC, ae.id DESC
        LIMIT $10
        "#,
    )
    .bind(workspace_id)
    .bind(before_created_at)
    .bind(normalize_optional_text(input.action))
    .bind(input.actor_user_id)
    .bind(input.actor_principal_id)
    .bind(normalize_optional_text(input.geo_network_kind))
    .bind(input.min_geo_risk_score)
    .bind(
        input
            .geo_risk_label
            .and_then(|label| normalize_optional_text(Some(label)))
            .map(|label| label.to_ascii_lowercase()),
    )
    .bind(normalize_optional_text(input.network_block_reason))
    .bind(limit)
    .fetch_all(db)
    .await?;

    rows.into_iter()
        .map(|row| {
            let metadata: sqlx::types::Json<serde_json::Value> = row.get("metadata");
            let geo_risk_labels: sqlx::types::Json<serde_json::Value> = row.get("geo_risk_labels");
            Ok(AuditEventView {
                id: row.get("id"),
                workspace_id: row.get("workspace_id"),
                actor_user_id: row.get("actor_user_id"),
                actor_principal_id: row.get("actor_principal_id"),
                actor_email: row.get("actor_email"),
                action: row.get("action"),
                target_type: row.get("target_type"),
                target_id: row.get("target_id"),
                ip: row.get("ip"),
                user_agent: row.get("user_agent"),
                geo_country_code: row.get("geo_country_code"),
                geo_network_kind: row.get("geo_network_kind"),
                geo_risk_score: row.get("geo_risk_score"),
                geo_risk_labels: geo_risk_labels
                    .0
                    .as_array()
                    .map(|labels| {
                        labels
                            .iter()
                            .filter_map(|label| label.as_str().map(ToOwned::to_owned))
                            .collect()
                    })
                    .unwrap_or_default(),
                network_block_reason: row.get("network_block_reason"),
                metadata: metadata.0,
                previous_event_hash: row.get("previous_event_hash"),
                event_hash: row.get("event_hash"),
                created_at: row.get("created_at"),
            })
        })
        .collect()
}
