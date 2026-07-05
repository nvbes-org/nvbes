use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, Transaction};
use thiserror::Error;

use super::{
    maxmind_csv::{
        GEOLITE_ASN_CSV_EDITION, GEOLITE_CITY_CSV_EDITION, GEOLITE_COUNTRY_CSV_EDITION,
        MaxMindGeoLiteCsvError, parse_geolite_csv_archives,
    },
    maxmind_download::{
        GEOLITE_DOWNLOAD_URL, MaxMindGeoLiteDownloadError, count_ranges, download_csv_archive,
        unzip_archive,
    },
    maxmind_types::{
        MAXMIND_GEOLITE_ASN_SOURCE_CODE, MAXMIND_GEOLITE_CITY_SOURCE_CODE,
        MAXMIND_GEOLITE_COUNTRY_SOURCE_CODE, MAXMIND_GEOLITE_SOURCE_CODES, MaxMindGeoLiteConfig,
        MaxMindGeoLiteConfigError, MaxMindGeoLiteRange,
    },
};

const GEOLITE_REFRESH_INTERVAL: &str = "3 days";
const GEOLITE_EXPIRES_AFTER: &str = "30 days";
const IMPORT_BATCH_SIZE: usize = 2_000;

#[derive(Debug, Error)]
pub enum MaxMindGeoLiteImportError {
    #[error("MaxMind GeoLite config is invalid")]
    Config(#[from] MaxMindGeoLiteConfigError),
    #[error("MaxMind GeoLite database download failed")]
    Download(#[from] reqwest::Error),
    #[error("MaxMind GeoLite archive extraction failed")]
    DownloadArchive(#[from] MaxMindGeoLiteDownloadError),
    #[error("MaxMind GeoLite csv parse failed")]
    Csv(#[from] MaxMindGeoLiteCsvError),
    #[error("MaxMind GeoLite database import failed")]
    Database(#[from] sqlx::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaxMindGeoLiteImportReport {
    pub downloaded: bool,
    pub imported_ranges: u64,
    pub expired_ranges: u64,
    pub country_ranges: u64,
    pub city_ranges: u64,
    pub asn_ranges: u64,
}

pub async fn run_scheduled_maxmind_geolite_import(
    pool: &PgPool,
    config: MaxMindGeoLiteConfig,
) -> Result<MaxMindGeoLiteImportReport, MaxMindGeoLiteImportError> {
    let config = config.enabled()?;
    if geolite_data_is_fresh(pool).await? {
        return Ok(MaxMindGeoLiteImportReport {
            downloaded: false,
            imported_ranges: 0,
            expired_ranges: 0,
            country_ranges: 0,
            city_ranges: 0,
            asn_ranges: 0,
        });
    }

    import_latest_maxmind_geolite(pool, config).await
}

pub async fn import_latest_maxmind_geolite(
    pool: &PgPool,
    config: MaxMindGeoLiteConfig,
) -> Result<MaxMindGeoLiteImportReport, MaxMindGeoLiteImportError> {
    let config = config.enabled()?;
    let client = Client::new();
    let country_archive =
        download_csv_archive(&client, &config, GEOLITE_COUNTRY_CSV_EDITION).await?;
    let city_archive = download_csv_archive(&client, &config, GEOLITE_CITY_CSV_EDITION).await?;
    let asn_archive = download_csv_archive(&client, &config, GEOLITE_ASN_CSV_EDITION).await?;
    let country_files = unzip_archive(&country_archive)?;
    let city_files = unzip_archive(&city_archive)?;
    let asn_files = unzip_archive(&asn_archive)?;
    let ranges = parse_geolite_csv_archives(&country_files, &city_files, &asn_files)?;
    let country_ranges = count_ranges(&ranges, MAXMIND_GEOLITE_COUNTRY_SOURCE_CODE);
    let city_ranges = count_ranges(&ranges, MAXMIND_GEOLITE_CITY_SOURCE_CODE);
    let asn_ranges = count_ranges(&ranges, MAXMIND_GEOLITE_ASN_SOURCE_CODE);

    let mut tx = pool.begin().await?;
    ensure_geolite_source_tx(&mut tx).await?;
    create_import_table_tx(&mut tx).await?;
    insert_import_rows_tx(&mut tx, &ranges).await?;
    let imported_ranges = upsert_imported_ranges_tx(&mut tx).await?;
    let expired_ranges = expire_missing_ranges_tx(&mut tx).await?;
    tx.commit().await?;

    Ok(MaxMindGeoLiteImportReport {
        downloaded: true,
        imported_ranges,
        expired_ranges,
        country_ranges,
        city_ranges,
        asn_ranges,
    })
}

async fn geolite_data_is_fresh(pool: &PgPool) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
          SELECT 1
          FROM geo_ip_network_relations
          WHERE source_code = ANY($1::TEXT[])
            AND fetched_at > now() - $2::interval
            AND (expires_at IS NULL OR expires_at > now())
        )
        "#,
    )
    .bind(MAXMIND_GEOLITE_SOURCE_CODES.as_slice())
    .bind(GEOLITE_REFRESH_INTERVAL)
    .fetch_one(pool)
    .await
}

async fn ensure_geolite_source_tx(tx: &mut Transaction<'_, Postgres>) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO geo_sources (code, kind, trust_level, priority, base_url)
        VALUES
          ($1, 'rdap', 'medium', 82, $4),
          ($2, 'rdap', 'medium', 81, $4),
          ($3, 'rdap', 'medium', 84, $4)
        ON CONFLICT (code) DO UPDATE SET
          kind = EXCLUDED.kind,
          trust_level = EXCLUDED.trust_level,
          priority = EXCLUDED.priority,
          base_url = EXCLUDED.base_url,
          enabled = TRUE,
          updated_at = now()
        "#,
    )
    .bind(MAXMIND_GEOLITE_COUNTRY_SOURCE_CODE)
    .bind(MAXMIND_GEOLITE_CITY_SOURCE_CODE)
    .bind(MAXMIND_GEOLITE_ASN_SOURCE_CODE)
    .bind("https://www.maxmind.com/en/geolite-free-ip-geolocation-data")
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn create_import_table_tx(tx: &mut Transaction<'_, Postgres>) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TEMP TABLE tmp_maxmind_geolite_import (
          relation_key TEXT PRIMARY KEY,
          source_code TEXT NOT NULL,
          edition_id TEXT NOT NULL,
          country_code TEXT,
          geoname_id TEXT,
          network CIDR NOT NULL,
          asn BIGINT,
          organization TEXT
        ) ON COMMIT DROP
        "#,
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn insert_import_rows_tx(
    tx: &mut Transaction<'_, Postgres>,
    ranges: &[MaxMindGeoLiteRange],
) -> Result<(), sqlx::Error> {
    for batch in ranges.chunks(IMPORT_BATCH_SIZE) {
        let relation_keys = batch.iter().map(relation_key).collect::<Vec<_>>();
        let source_codes = batch
            .iter()
            .map(|range| range.source_code.to_string())
            .collect::<Vec<_>>();
        let edition_ids = batch
            .iter()
            .map(|range| range.edition_id.to_string())
            .collect::<Vec<_>>();
        let country_codes = batch
            .iter()
            .map(|range| range.country_code.clone())
            .collect::<Vec<_>>();
        let geoname_ids = batch
            .iter()
            .map(|range| range.geoname_id.clone())
            .collect::<Vec<_>>();
        let networks = batch
            .iter()
            .map(|range| range.network.to_string())
            .collect::<Vec<_>>();
        let asns = batch.iter().map(|range| range.asn).collect::<Vec<_>>();
        let organizations = batch
            .iter()
            .map(|range| range.organization.clone())
            .collect::<Vec<_>>();

        sqlx::query(
            r#"
            INSERT INTO tmp_maxmind_geolite_import (
              relation_key, source_code, edition_id, country_code,
              geoname_id, network, asn, organization
            )
            SELECT *
            FROM UNNEST(
              $1::TEXT[], $2::TEXT[], $3::TEXT[], $4::TEXT[],
              $5::TEXT[], $6::CIDR[], $7::BIGINT[], $8::TEXT[]
            )
            ON CONFLICT (relation_key) DO UPDATE SET
              source_code = EXCLUDED.source_code,
              edition_id = EXCLUDED.edition_id,
              country_code = EXCLUDED.country_code,
              geoname_id = EXCLUDED.geoname_id,
              network = EXCLUDED.network,
              asn = EXCLUDED.asn,
              organization = EXCLUDED.organization
            "#,
        )
        .bind(&relation_keys)
        .bind(&source_codes)
        .bind(&edition_ids)
        .bind(&country_codes)
        .bind(&geoname_ids)
        .bind(&networks)
        .bind(&asns)
        .bind(&organizations)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

async fn upsert_imported_ranges_tx(tx: &mut Transaction<'_, Postgres>) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        INSERT INTO geo_ip_network_relations (
          source_code, relation_key, registry, network, asn, organization, country_code,
          source_reference, network_kind, risk_score, risk_labels, raw_payload,
          fetched_at, expires_at
        )
        SELECT
          source_code, relation_key, 'maxmind', network, asn, organization, country_code,
          $1, 'unknown', 30, ARRAY['source:maxmind_geolite']::TEXT[],
          jsonb_build_object(
            'edition', edition_id,
            'geoname_id', geoname_id,
            'network', network::TEXT,
            'asn', asn,
            'organization', organization
          ),
          statement_timestamp(), now() + $2::interval
        FROM tmp_maxmind_geolite_import
        ON CONFLICT (source_code, relation_key)
        DO UPDATE SET
          network = EXCLUDED.network,
          asn = EXCLUDED.asn,
          organization = EXCLUDED.organization,
          country_code = EXCLUDED.country_code,
          source_reference = EXCLUDED.source_reference,
          risk_labels = EXCLUDED.risk_labels,
          raw_payload = EXCLUDED.raw_payload,
          fetched_at = statement_timestamp(),
          expires_at = EXCLUDED.expires_at
        "#,
    )
    .bind(GEOLITE_DOWNLOAD_URL)
    .bind(GEOLITE_EXPIRES_AFTER)
    .execute(&mut **tx)
    .await?;
    Ok(result.rows_affected())
}

async fn expire_missing_ranges_tx(tx: &mut Transaction<'_, Postgres>) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        UPDATE geo_ip_network_relations relation
        SET expires_at = statement_timestamp()
        WHERE relation.source_code = ANY($1::TEXT[])
          AND (relation.expires_at IS NULL OR relation.expires_at > now())
          AND NOT EXISTS (
              SELECT 1
              FROM tmp_maxmind_geolite_import imported
              WHERE imported.relation_key = relation.relation_key
          )
        "#,
    )
    .bind(MAXMIND_GEOLITE_SOURCE_CODES.as_slice())
    .execute(&mut **tx)
    .await?;
    Ok(result.rows_affected())
}

fn relation_key(range: &MaxMindGeoLiteRange) -> String {
    format!("{}:{}", range.source_code, range.network)
}
