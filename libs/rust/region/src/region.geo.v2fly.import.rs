use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Transaction};
use thiserror::Error;

use super::v2fly_dat::{V2flyGeoIpDatError, V2flyGeoIpRange, parse_v2fly_geoip_dat};

pub const V2FLY_GEOIP_SOURCE_CODE: &str = "v2fly_geoip";
const V2FLY_GEOIP_DAT_URL: &str =
    "https://github.com/v2fly/geoip/releases/latest/download/geoip.dat";
const V2FLY_GEOIP_SHA256_URL: &str =
    "https://github.com/v2fly/geoip/releases/latest/download/geoip.dat.sha256sum";
const V2FLY_GEOIP_REFRESH_INTERVAL: &str = "7 days";
const V2FLY_GEOIP_EXPIRES_AFTER: &str = "45 days";
const IMPORT_BATCH_SIZE: usize = 2_000;

#[derive(Debug, Error)]
pub enum V2flyGeoIpImportError {
    #[error("v2fly geoip download failed")]
    Download(#[from] reqwest::Error),
    #[error("v2fly geoip checksum file is invalid")]
    InvalidChecksum,
    #[error("v2fly geoip checksum mismatch")]
    ChecksumMismatch,
    #[error("v2fly geoip dat parse failed")]
    Parse(#[from] V2flyGeoIpDatError),
    #[error("v2fly geoip database import failed")]
    Database(#[from] sqlx::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct V2flyGeoIpImportReport {
    pub downloaded: bool,
    pub imported_ranges: u64,
    pub expired_ranges: u64,
}

pub async fn run_scheduled_v2fly_geoip_import(
    pool: &PgPool,
) -> Result<V2flyGeoIpImportReport, V2flyGeoIpImportError> {
    if v2fly_geoip_data_is_fresh(pool).await? {
        return Ok(V2flyGeoIpImportReport {
            downloaded: false,
            imported_ranges: 0,
            expired_ranges: 0,
        });
    }

    import_latest_v2fly_geoip(pool).await
}

pub async fn import_latest_v2fly_geoip(
    pool: &PgPool,
) -> Result<V2flyGeoIpImportReport, V2flyGeoIpImportError> {
    let client = Client::new();
    let checksum = download_text(&client, V2FLY_GEOIP_SHA256_URL).await?;
    let expected_checksum = parse_sha256sum(&checksum)?;
    let dat = download_bytes(&client, V2FLY_GEOIP_DAT_URL).await?;
    verify_sha256(&dat, expected_checksum)?;
    let ranges = parse_v2fly_geoip_dat(&dat)?;

    let mut tx = pool.begin().await?;
    ensure_v2fly_geoip_source_tx(&mut tx).await?;
    create_import_table_tx(&mut tx).await?;
    insert_import_rows_tx(&mut tx, &ranges).await?;
    let imported_ranges = upsert_imported_ranges_tx(&mut tx).await?;
    let expired_ranges = expire_missing_ranges_tx(&mut tx).await?;
    tx.commit().await?;

    Ok(V2flyGeoIpImportReport {
        downloaded: true,
        imported_ranges,
        expired_ranges,
    })
}

async fn v2fly_geoip_data_is_fresh(pool: &PgPool) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
          SELECT 1
          FROM geo_ip_network_relations
          WHERE source_code = $1
            AND fetched_at > now() - $2::interval
            AND (expires_at IS NULL OR expires_at > now())
        )
        "#,
    )
    .bind(V2FLY_GEOIP_SOURCE_CODE)
    .bind(V2FLY_GEOIP_REFRESH_INTERVAL)
    .fetch_one(pool)
    .await
}

async fn ensure_v2fly_geoip_source_tx(
    tx: &mut Transaction<'_, Postgres>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO geo_sources (code, kind, trust_level, priority, base_url)
        VALUES ($1, 'rdap', 'medium', 85, $2)
        ON CONFLICT (code) DO UPDATE SET
          kind = EXCLUDED.kind,
          trust_level = EXCLUDED.trust_level,
          priority = EXCLUDED.priority,
          base_url = EXCLUDED.base_url,
          enabled = TRUE,
          updated_at = now()
        "#,
    )
    .bind(V2FLY_GEOIP_SOURCE_CODE)
    .bind("https://github.com/v2fly/geoip")
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn create_import_table_tx(tx: &mut Transaction<'_, Postgres>) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TEMP TABLE tmp_v2fly_geoip_import (
          relation_key TEXT PRIMARY KEY,
          country_code TEXT NOT NULL,
          network CIDR NOT NULL
        ) ON COMMIT DROP
        "#,
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn insert_import_rows_tx(
    tx: &mut Transaction<'_, Postgres>,
    ranges: &[V2flyGeoIpRange],
) -> Result<(), sqlx::Error> {
    for batch in ranges.chunks(IMPORT_BATCH_SIZE) {
        let relation_keys = batch.iter().map(relation_key).collect::<Vec<_>>();
        let country_codes = batch
            .iter()
            .map(|range| range.country_code.clone())
            .collect::<Vec<_>>();
        let networks = batch
            .iter()
            .map(|range| range.network.to_string())
            .collect::<Vec<_>>();

        sqlx::query(
            r#"
            INSERT INTO tmp_v2fly_geoip_import (relation_key, country_code, network)
            SELECT *
            FROM UNNEST($1::TEXT[], $2::TEXT[], $3::CIDR[])
            ON CONFLICT (relation_key) DO UPDATE SET
              country_code = EXCLUDED.country_code,
              network = EXCLUDED.network
            "#,
        )
        .bind(&relation_keys)
        .bind(&country_codes)
        .bind(&networks)
        .execute(&mut **tx)
        .await?;
    }

    Ok(())
}

async fn upsert_imported_ranges_tx(tx: &mut Transaction<'_, Postgres>) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        INSERT INTO geo_ip_network_relations (
          source_code, relation_key, registry, network, country_code,
          source_reference, network_kind, risk_score, risk_labels, raw_payload,
          fetched_at, expires_at
        )
        SELECT
          $1, relation_key, 'v2fly', network, country_code,
          $2, 'unknown', 35, ARRAY['source:v2fly_geoip']::TEXT[],
          jsonb_build_object('source', $1, 'network', network::TEXT),
          statement_timestamp(), now() + $3::interval
        FROM tmp_v2fly_geoip_import
        ON CONFLICT (source_code, relation_key)
        DO UPDATE SET
          registry = EXCLUDED.registry,
          network = EXCLUDED.network,
          country_code = EXCLUDED.country_code,
          source_reference = EXCLUDED.source_reference,
          network_kind = EXCLUDED.network_kind,
          risk_score = EXCLUDED.risk_score,
          risk_labels = EXCLUDED.risk_labels,
          raw_payload = EXCLUDED.raw_payload,
          fetched_at = statement_timestamp(),
          expires_at = EXCLUDED.expires_at
        "#,
    )
    .bind(V2FLY_GEOIP_SOURCE_CODE)
    .bind(V2FLY_GEOIP_DAT_URL)
    .bind(V2FLY_GEOIP_EXPIRES_AFTER)
    .execute(&mut **tx)
    .await?;
    Ok(result.rows_affected())
}

async fn expire_missing_ranges_tx(tx: &mut Transaction<'_, Postgres>) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        UPDATE geo_ip_network_relations relation
        SET expires_at = statement_timestamp()
        WHERE relation.source_code = $1
          AND (relation.expires_at IS NULL OR relation.expires_at > now())
          AND NOT EXISTS (
              SELECT 1
              FROM tmp_v2fly_geoip_import imported
              WHERE imported.relation_key = relation.relation_key
          )
        "#,
    )
    .bind(V2FLY_GEOIP_SOURCE_CODE)
    .execute(&mut **tx)
    .await?;
    Ok(result.rows_affected())
}

async fn download_text(client: &Client, url: &str) -> Result<String, reqwest::Error> {
    client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await
}

async fn download_bytes(client: &Client, url: &str) -> Result<Vec<u8>, reqwest::Error> {
    Ok(client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?
        .to_vec())
}

fn parse_sha256sum(body: &str) -> Result<[u8; 32], V2flyGeoIpImportError> {
    let hash = body
        .split_whitespace()
        .next()
        .ok_or(V2flyGeoIpImportError::InvalidChecksum)?;
    decode_sha256_hex(hash).ok_or(V2flyGeoIpImportError::InvalidChecksum)
}

fn verify_sha256(bytes: &[u8], expected: [u8; 32]) -> Result<(), V2flyGeoIpImportError> {
    let actual = Sha256::digest(bytes);
    if actual.as_slice() == expected {
        Ok(())
    } else {
        Err(V2flyGeoIpImportError::ChecksumMismatch)
    }
}

fn decode_sha256_hex(value: &str) -> Option<[u8; 32]> {
    if value.len() != 64 {
        return None;
    }
    let mut bytes = [0_u8; 32];
    for index in 0..32 {
        bytes[index] = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16).ok()?;
    }
    Some(bytes)
}

fn relation_key(range: &V2flyGeoIpRange) -> String {
    format!("{}:{}", range.country_code, range.network)
}

#[cfg(test)]
mod tests {
    use super::{decode_sha256_hex, parse_sha256sum};

    #[test]
    fn parses_sha256sum_file() {
        let checksum =
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef  geoip.dat";

        assert!(parse_sha256sum(checksum).is_ok());
    }

    #[test]
    fn rejects_invalid_sha256sum() {
        assert!(decode_sha256_hex("abc").is_none());
        assert!(parse_sha256sum("").is_err());
    }
}
