use reqwest::Client;
use sqlx::{PgPool, Postgres, Transaction};
use thiserror::Error;

use super::loyalsoldier_download::{
    LoyalsoldierGeoIpDownloadError, download_verified_loyalsoldier_geoip,
};
use super::loyalsoldier_types::{
    LOYALSOLDIER_GEOIP_DAT_URL, LOYALSOLDIER_GEOIP_SOURCE_CODE, LoyalsoldierGeoIpImportReport,
    loyalsoldier_is_country, map_loyalsoldier_range,
};
use super::v2fly_dat::{V2flyGeoIpDatEntry, V2flyGeoIpDatError, parse_v2fly_geoip_dat_entries};

const LOYALSOLDIER_GEOIP_REFRESH_INTERVAL: &str = "7 days";
const LOYALSOLDIER_GEOIP_EXPIRES_AFTER: &str = "21 days";
const IMPORT_BATCH_SIZE: usize = 2_000;

#[derive(Debug, Error)]
pub enum LoyalsoldierGeoIpImportError {
    #[error("Loyalsoldier GeoIP download or checksum verification failed")]
    Download(#[from] LoyalsoldierGeoIpDownloadError),
    #[error("Loyalsoldier GeoIP dat parse failed")]
    Parse(#[from] V2flyGeoIpDatError),
    #[error("Loyalsoldier GeoIP database import failed")]
    Database(#[from] sqlx::Error),
}

pub async fn run_scheduled_loyalsoldier_geoip_import(
    pool: &PgPool,
) -> Result<LoyalsoldierGeoIpImportReport, LoyalsoldierGeoIpImportError> {
    if loyalsoldier_geoip_data_is_fresh(pool).await? {
        return Ok(LoyalsoldierGeoIpImportReport {
            downloaded: false,
            imported_ranges: 0,
            expired_ranges: 0,
            country_ranges: 0,
            category_ranges: 0,
        });
    }

    import_latest_loyalsoldier_geoip(pool).await
}

pub async fn import_latest_loyalsoldier_geoip(
    pool: &PgPool,
) -> Result<LoyalsoldierGeoIpImportReport, LoyalsoldierGeoIpImportError> {
    import_latest_loyalsoldier_geoip_with_client(pool, Client::new()).await
}

pub(crate) async fn import_latest_loyalsoldier_geoip_with_client(
    pool: &PgPool,
    client: Client,
) -> Result<LoyalsoldierGeoIpImportReport, LoyalsoldierGeoIpImportError> {
    let dat = download_verified_loyalsoldier_geoip(&client).await?;
    let ranges = parse_v2fly_geoip_dat_entries(&dat)?;
    persist_loyalsoldier_geoip_entries(pool, &ranges).await
}

async fn persist_loyalsoldier_geoip_entries(
    pool: &PgPool,
    ranges: &[V2flyGeoIpDatEntry],
) -> Result<LoyalsoldierGeoIpImportReport, LoyalsoldierGeoIpImportError> {
    let country_ranges = ranges
        .iter()
        .filter(|range| loyalsoldier_is_country(&range.code))
        .count() as u64;
    let category_ranges = ranges.len() as u64 - country_ranges;

    let mut tx = pool.begin().await?;
    ensure_loyalsoldier_geoip_source_tx(&mut tx).await?;
    create_import_table_tx(&mut tx).await?;
    insert_import_rows_tx(&mut tx, ranges).await?;
    let imported_ranges = upsert_imported_ranges_tx(&mut tx).await?;
    let expired_ranges = expire_missing_ranges_tx(&mut tx).await?;
    tx.commit().await?;

    Ok(LoyalsoldierGeoIpImportReport {
        downloaded: true,
        imported_ranges,
        expired_ranges,
        country_ranges,
        category_ranges,
    })
}

async fn loyalsoldier_geoip_data_is_fresh(pool: &PgPool) -> Result<bool, sqlx::Error> {
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
    .bind(LOYALSOLDIER_GEOIP_SOURCE_CODE)
    .bind(LOYALSOLDIER_GEOIP_REFRESH_INTERVAL)
    .fetch_one(pool)
    .await
}

async fn ensure_loyalsoldier_geoip_source_tx(
    tx: &mut Transaction<'_, Postgres>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO geo_sources (code, kind, trust_level, priority, base_url)
        VALUES ($1, 'rdap', 'medium', 86, $2)
        ON CONFLICT (code) DO UPDATE SET
          kind = EXCLUDED.kind,
          trust_level = EXCLUDED.trust_level,
          priority = EXCLUDED.priority,
          base_url = EXCLUDED.base_url,
          enabled = TRUE,
          updated_at = now()
        "#,
    )
    .bind(LOYALSOLDIER_GEOIP_SOURCE_CODE)
    .bind("https://github.com/Loyalsoldier/geoip")
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn create_import_table_tx(tx: &mut Transaction<'_, Postgres>) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TEMP TABLE tmp_loyalsoldier_geoip_import (
          relation_key TEXT PRIMARY KEY,
          code TEXT NOT NULL,
          country_code TEXT,
          network CIDR NOT NULL,
          network_kind TEXT NOT NULL,
          risk_score SMALLINT NOT NULL,
          risk_labels TEXT[] NOT NULL
        ) ON COMMIT DROP
        "#,
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn insert_import_rows_tx(
    tx: &mut Transaction<'_, Postgres>,
    ranges: &[V2flyGeoIpDatEntry],
) -> Result<(), sqlx::Error> {
    for batch in ranges.chunks(IMPORT_BATCH_SIZE) {
        let mapped = batch.iter().map(map_loyalsoldier_range).collect::<Vec<_>>();
        let keys = mapped
            .iter()
            .map(|range| range.key.clone())
            .collect::<Vec<_>>();
        let codes = mapped
            .iter()
            .map(|range| range.code.clone())
            .collect::<Vec<_>>();
        let countries = mapped
            .iter()
            .map(|range| range.country_code.clone())
            .collect::<Vec<_>>();
        let networks = mapped
            .iter()
            .map(|range| range.network.clone())
            .collect::<Vec<_>>();
        let kinds = mapped
            .iter()
            .map(|range| range.kind.clone())
            .collect::<Vec<_>>();
        let scores = mapped.iter().map(|range| range.score).collect::<Vec<_>>();
        let labels = mapped
            .iter()
            .map(|range| range.labels.join("\u{1f}"))
            .collect::<Vec<_>>();

        sqlx::query(
            r#"
            INSERT INTO tmp_loyalsoldier_geoip_import (
              relation_key, code, country_code, network, network_kind, risk_score, risk_labels
            )
            SELECT
              relation_key, code, country_code, network, network_kind, risk_score,
              string_to_array(labels_text, chr(31))
            FROM UNNEST($1::TEXT[], $2::TEXT[], $3::TEXT[], $4::CIDR[], $5::TEXT[], $6::INT2[], $7::TEXT[])
              AS imported(relation_key, code, country_code, network, network_kind, risk_score, labels_text)
            ON CONFLICT (relation_key) DO UPDATE SET
              code = EXCLUDED.code,
              country_code = EXCLUDED.country_code,
              network = EXCLUDED.network,
              network_kind = EXCLUDED.network_kind,
              risk_score = EXCLUDED.risk_score,
              risk_labels = EXCLUDED.risk_labels
            "#,
        )
        .bind(&keys)
        .bind(&codes)
        .bind(&countries)
        .bind(&networks)
        .bind(&kinds)
        .bind(&scores)
        .bind(&labels)
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
          $1, relation_key, 'loyalsoldier', network, country_code,
          $2, network_kind, risk_score, risk_labels,
          jsonb_build_object('source', $1, 'category', code, 'network', network::TEXT),
          statement_timestamp(), now() + $3::interval
        FROM tmp_loyalsoldier_geoip_import
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
    .bind(LOYALSOLDIER_GEOIP_SOURCE_CODE)
    .bind(LOYALSOLDIER_GEOIP_DAT_URL)
    .bind(LOYALSOLDIER_GEOIP_EXPIRES_AFTER)
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
              FROM tmp_loyalsoldier_geoip_import imported
              WHERE imported.relation_key = relation.relation_key
          )
        "#,
    )
    .bind(LOYALSOLDIER_GEOIP_SOURCE_CODE)
    .execute(&mut **tx)
    .await?;
    Ok(result.rows_affected())
}

#[cfg(test)]
#[path = "region.geo.loyalsoldier.import.tests.rs"]
mod tests;
