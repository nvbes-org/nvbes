use serde_json::Value;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::http::error::AppError;

use super::super::types::{ListSecurityEventsInput, RiskEventView};

#[derive(Debug, Default)]
pub(super) struct SecurityEventFilters {
    pub(super) geo_country_code: Option<String>,
    pub(super) geo_source: Option<String>,
    pub(super) geo_confidence: Option<String>,
}

impl SecurityEventFilters {
    pub(super) fn from_input(input: &ListSecurityEventsInput) -> Self {
        Self {
            geo_country_code: normalize_text_filter(input.geo_country_code.as_deref())
                .map(str::to_ascii_uppercase),
            geo_source: normalize_text_filter(input.geo_source.as_deref()).map(ToOwned::to_owned),
            geo_confidence: normalize_text_filter(input.geo_confidence.as_deref())
                .map(ToOwned::to_owned),
        }
    }
}

pub(super) async fn fetch_risk_events(
    db: &PgPool,
    workspace_id: Uuid,
    before_id: Option<Uuid>,
    limit: i64,
    filters: &SecurityEventFilters,
) -> Result<Vec<RiskEventView>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT
          id, principal_id, session_id, device_id, event_type, ip_address::text AS ip_address,
          user_agent, risk_score, risk_factors, decision, metadata,
          COALESCE(
            risk_factors->>'geo_country_code',
            risk_factors #>> '{geo,country_code}',
            metadata #>> '{geo,geo_country_code}',
            metadata->>'geo_country_code'
          ) AS geo_country_code,
          COALESCE(
            risk_factors->>'geo_source',
            risk_factors #>> '{geo,source}',
            metadata #>> '{geo,geo_source}',
            metadata->>'geo_source'
          ) AS geo_source,
          COALESCE(
            risk_factors->>'geo_confidence',
            risk_factors #>> '{geo,confidence}',
            metadata #>> '{geo,geo_confidence}',
            metadata->>'geo_confidence'
          ) AS geo_confidence,
          created_at
        FROM risk_events
        WHERE principal_id IN (
          SELECT principal_id
          FROM principals
          WHERE tenant_id = (SELECT tenant_id FROM workspaces WHERE id = $1)
        )
          AND ($2::uuid IS NULL OR id < $2)
          AND (
            $4::text IS NULL OR upper(COALESCE(
              risk_factors->>'geo_country_code',
              risk_factors #>> '{geo,country_code}',
              metadata #>> '{geo,geo_country_code}',
              metadata->>'geo_country_code'
            )) = upper($4)
          )
          AND (
            $5::text IS NULL OR COALESCE(
              risk_factors->>'geo_source',
              risk_factors #>> '{geo,source}',
              metadata #>> '{geo,geo_source}',
              metadata->>'geo_source'
            ) = $5
          )
          AND (
            $6::text IS NULL OR COALESCE(
              risk_factors->>'geo_confidence',
              risk_factors #>> '{geo,confidence}',
              metadata #>> '{geo,geo_confidence}',
              metadata->>'geo_confidence'
            ) = $6
          )
        ORDER BY created_at DESC, id DESC
        LIMIT $3
        "#,
    )
    .bind(workspace_id)
    .bind(before_id)
    .bind(limit)
    .bind(filters.geo_country_code.as_deref())
    .bind(filters.geo_source.as_deref())
    .bind(filters.geo_confidence.as_deref())
    .fetch_all(db)
    .await?;

    rows.into_iter()
        .map(|row| {
            let risk_factors: sqlx::types::Json<Value> = row.get("risk_factors");
            let metadata: sqlx::types::Json<Value> = row.get("metadata");
            Ok(RiskEventView {
                id: row.get("id"),
                principal_id: row.get("principal_id"),
                session_id: row.get("session_id"),
                device_id: row.get("device_id"),
                event_type: row.get("event_type"),
                ip_address: row.get("ip_address"),
                user_agent: row.get("user_agent"),
                risk_score: row.get("risk_score"),
                risk_factors: risk_factors.0,
                decision: row.get("decision"),
                metadata: metadata.0,
                geo_country_code: row.get("geo_country_code"),
                geo_source: row.get("geo_source"),
                geo_confidence: row.get("geo_confidence"),
                created_at: row.get("created_at"),
            })
        })
        .collect()
}

fn normalize_text_filter(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}
