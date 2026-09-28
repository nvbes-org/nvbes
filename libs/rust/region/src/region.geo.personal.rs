use chrono::{DateTime, Utc};
use ipnet::IpNet;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use thiserror::Error;
use uuid::Uuid;

use crate::geo::database::PersonalGeoDatabaseError;
use crate::geo::types::{GeoLocation, GeoNetworkKind};

#[derive(Debug, Error)]
pub enum PersonalGeoStoreError {
    #[error(transparent)]
    Validation(#[from] PersonalGeoDatabaseError),
    #[error("personal geo range priority must be greater than zero")]
    InvalidPriority,
    #[error("personal geo range risk score must be between 0 and 100")]
    InvalidRiskScore,
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpsertPersonalGeoRangeInput {
    pub network: IpNet,
    pub country_code: String,
    pub priority: i16,
    pub source_reference: Option<String>,
    pub note: Option<String>,
    pub network_kind: GeoNetworkKind,
    pub risk_score: u8,
    pub risk_labels: Vec<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, FromRow)]
pub struct PersonalGeoRangeView {
    pub id: Uuid,
    pub network: String,
    pub country_code: String,
    pub priority: i16,
    pub source_reference: Option<String>,
    pub note: Option<String>,
    pub network_kind: String,
    pub risk_score: i16,
    pub risk_labels: Vec<String>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

pub async fn list_personal_geo_ranges(
    pool: &PgPool,
) -> Result<Vec<PersonalGeoRangeView>, sqlx::Error> {
    sqlx::query_as::<_, PersonalGeoRangeView>(PERSONAL_GEO_RANGE_SELECT)
        .fetch_all(pool)
        .await
}

pub async fn upsert_personal_geo_range(
    pool: &PgPool,
    input: UpsertPersonalGeoRangeInput,
) -> Result<PersonalGeoRangeView, PersonalGeoStoreError> {
    let mut tx = pool.begin().await?;
    let view = upsert_personal_geo_range_tx(&mut tx, input).await?;
    tx.commit().await?;
    Ok(view)
}

pub async fn upsert_personal_geo_range_tx(
    tx: &mut Transaction<'_, Postgres>,
    input: UpsertPersonalGeoRangeInput,
) -> Result<PersonalGeoRangeView, PersonalGeoStoreError> {
    validate_personal_geo_input(&input)?;

    let row = sqlx::query_as::<_, PersonalGeoRangeView>(
        r#"
        INSERT INTO geo_personal_ip_ranges (
          network, country_code, priority, source_reference, note, enabled, expires_at
          , network_kind, risk_score, risk_labels
        )
        VALUES ($1::cidr, $2, $3, $4, $5, TRUE, $6, $7, $8, $9)
        ON CONFLICT (network)
        DO UPDATE SET
          country_code = EXCLUDED.country_code,
          priority = EXCLUDED.priority,
          source_reference = EXCLUDED.source_reference,
          note = EXCLUDED.note,
          enabled = TRUE,
          expires_at = EXCLUDED.expires_at,
          network_kind = EXCLUDED.network_kind,
          risk_score = EXCLUDED.risk_score,
          risk_labels = EXCLUDED.risk_labels,
          updated_at = NOW()
        RETURNING
          id, network::text AS network, country_code, priority, source_reference,
          note, network_kind, risk_score, risk_labels, enabled, created_at, updated_at, expires_at
        "#,
    )
    .bind(input.network.to_string())
    .bind(input.country_code.to_ascii_uppercase())
    .bind(input.priority)
    .bind(input.source_reference)
    .bind(input.note)
    .bind(input.expires_at)
    .bind(input.network_kind.as_str())
    .bind(i16::from(input.risk_score))
    .bind(input.risk_labels)
    .fetch_one(&mut **tx)
    .await?;

    Ok(row)
}

pub async fn disable_personal_geo_range(
    pool: &PgPool,
    network: IpNet,
) -> Result<Option<PersonalGeoRangeView>, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let view = disable_personal_geo_range_tx(&mut tx, network).await?;
    tx.commit().await?;
    Ok(view)
}

pub async fn disable_personal_geo_range_tx(
    tx: &mut Transaction<'_, Postgres>,
    network: IpNet,
) -> Result<Option<PersonalGeoRangeView>, sqlx::Error> {
    sqlx::query_as::<_, PersonalGeoRangeView>(
        r#"
        UPDATE geo_personal_ip_ranges
        SET enabled = FALSE,
            updated_at = NOW()
        WHERE network = $1::cidr
        RETURNING
          id, network::text AS network, country_code, priority, source_reference,
          note, network_kind, risk_score, risk_labels, enabled, created_at, updated_at, expires_at
        "#,
    )
    .bind(network.to_string())
    .fetch_optional(&mut **tx)
    .await
}

fn validate_personal_geo_input(
    input: &UpsertPersonalGeoRangeInput,
) -> Result<(), PersonalGeoStoreError> {
    if input.priority <= 0 {
        return Err(PersonalGeoStoreError::InvalidPriority);
    }
    if input.risk_score > 100 {
        return Err(PersonalGeoStoreError::InvalidRiskScore);
    }
    GeoLocation::from_country_code(&input.country_code)
        .ok_or(PersonalGeoDatabaseError::UnsupportedCountryCode)?;
    Ok(())
}

const PERSONAL_GEO_RANGE_SELECT: &str = r#"
        SELECT
          id, network::text AS network, country_code, priority, source_reference,
          note, network_kind, risk_score, risk_labels, enabled, created_at, updated_at, expires_at
        FROM geo_personal_ip_ranges
        ORDER BY enabled DESC, priority ASC, masklen(network) DESC, updated_at DESC
        "#;

#[cfg(test)]
#[path = "region.geo.personal.tests.rs"]
mod tests;
