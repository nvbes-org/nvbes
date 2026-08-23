use nvbes_core::pagination::{KeysetCursor, page_from_rows};
use sqlx::{PgPool, Postgres, Row, Transaction};
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
    let cursor = input
        .cursor
        .as_deref()
        .map(KeysetCursor::decode)
        .transpose()
        .map_err(|_| AppError::bad_request("invalid_cursor", "Pagination cursor is invalid."))?;
    let rows = fetch_events(db, access.workspace_id, input, cursor, limit + 1).await?;
    let page = page_from_rows(rows, limit as usize, |event| KeysetCursor {
        created_at: event.created_at,
        id: event.id,
    });

    Ok(AuditEventsResponse {
        workspace_id: access.workspace_id,
        events: page.items,
        next_cursor: page
            .next_cursor
            .map(KeysetCursor::encode)
            .transpose()
            .map_err(|_| {
                AppError::internal(
                    "cursor_encoding_failed",
                    "Pagination cursor could not be encoded.",
                )
            })?,
        has_more: page.has_more,
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
            cursor: None,
            action: None,
            actor_user_id: None,
            actor_principal_id: None,
            geo_network_kind: None,
            min_geo_risk_score: None,
            geo_risk_label: None,
            network_block_reason: None,
        },
        None,
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
    record_event_tx(&mut tx, input).await?;
    tx.commit().await?;

    Ok(())
}

pub async fn record_event_tx(
    tx: &mut Transaction<'_, Postgres>,
    input: AuditRecordInput<'_>,
) -> Result<(), AppError> {
    let metadata = crate::domains::audit::geo::enrich_audit_metadata_tx(
        tx,
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
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn fetch_events(
    db: &PgPool,
    workspace_id: Uuid,
    input: ListAuditEventsInput,
    cursor: Option<KeysetCursor>,
    limit: i64,
) -> Result<Vec<AuditEventView>, AppError> {
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
          AND ($2::timestamptz IS NULL OR (ae.created_at, ae.id) < ($2, $3))
          AND ($4::text IS NULL OR ae.action = $4)
          AND ($5::uuid IS NULL OR ae.actor_user_id = $5)
          AND ($6::uuid IS NULL OR ae.actor_principal_id = $6)
          AND ($7::text IS NULL OR ae.metadata #>> '{geo,geo_network_kind}' = $7)
          AND ($8::bigint IS NULL OR COALESCE(NULLIF(ae.metadata #>> '{geo,geo_risk_score}', '')::bigint, 0) >= $8)
          AND (
            $9::text IS NULL OR EXISTS (
              SELECT 1
              FROM jsonb_array_elements_text(COALESCE(ae.metadata #> '{geo,geo_risk_labels}', '[]'::jsonb)) AS label(value)
              WHERE lower(label.value) = $9
            )
          )
          AND ($10::text IS NULL OR ae.metadata->>'network_block_reason' = $10)
        ORDER BY ae.created_at DESC, ae.id DESC
        LIMIT $11
        "#,
    )
    .bind(workspace_id)
    .bind(cursor.map(|value| value.created_at))
    .bind(cursor.map(|value| value.id))
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
