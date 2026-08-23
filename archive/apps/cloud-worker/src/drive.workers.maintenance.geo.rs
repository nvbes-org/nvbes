use serde_json::Value as JsonValue;

use crate::db::Database;

pub const JOB_GEO_LOOKUP_MAINTENANCE: &str = "geo.lookup_maintenance";
pub const JOB_GEO_V2FLY_IMPORT: &str = "geo.v2fly_import";
pub const JOB_GEO_MAXMIND_GEOLITE_IMPORT: &str = "geo.maxmind_geolite_import";
pub const JOB_GEO_LOYALSOLDIER_IMPORT: &str = "geo.loyalsoldier_import";

pub async fn run_geo_lookup_maintenance(database: &Database) -> anyhow::Result<JsonValue> {
    let report = nvbes_region::geo::run_geo_maintenance(database).await?;

    Ok(serde_json::json!({
        "expired_personal_ranges_disabled": report.expired_personal_ranges_disabled,
        "expired_unreferenced_relations_deleted": report.expired_unreferenced_relations_deleted,
        "expired_maxmind_evidence_scrubbed": report.expired_maxmind_evidence_scrubbed,
        "expired_maxmind_relations_deleted": report.expired_maxmind_relations_deleted
    }))
}

pub async fn import_v2fly_geoip(database: &Database) -> anyhow::Result<JsonValue> {
    let report = nvbes_region::geo::run_scheduled_v2fly_geoip_import(database).await?;

    Ok(serde_json::json!({
        "downloaded": report.downloaded,
        "imported_ranges": report.imported_ranges,
        "expired_ranges": report.expired_ranges
    }))
}

pub async fn import_maxmind_geolite(
    config: &nvbes_core::config::AppConfig,
    database: &Database,
) -> anyhow::Result<JsonValue> {
    if !config.maxmind_geolite_database_enabled {
        return Ok(serde_json::json!({
            "enabled": false,
            "downloaded": false,
            "imported_ranges": 0,
            "expired_ranges": 0
        }));
    }

    let report = nvbes_region::geo::run_scheduled_maxmind_geolite_import(
        database,
        maxmind_geolite_config(config)?,
    )
    .await?;

    Ok(serde_json::json!({
        "enabled": true,
        "downloaded": report.downloaded,
        "imported_ranges": report.imported_ranges,
        "expired_ranges": report.expired_ranges,
        "country_ranges": report.country_ranges,
        "city_ranges": report.city_ranges,
        "asn_ranges": report.asn_ranges
    }))
}

pub async fn import_loyalsoldier_geoip(
    config: &nvbes_core::config::AppConfig,
    database: &Database,
) -> anyhow::Result<JsonValue> {
    if !config.loyalsoldier_geoip_enabled {
        return Ok(serde_json::json!({
            "enabled": false,
            "downloaded": false,
            "imported_ranges": 0,
            "expired_ranges": 0
        }));
    }

    let report = nvbes_region::geo::run_scheduled_loyalsoldier_geoip_import(database).await?;

    Ok(serde_json::json!({
        "enabled": true,
        "downloaded": report.downloaded,
        "imported_ranges": report.imported_ranges,
        "expired_ranges": report.expired_ranges,
        "country_ranges": report.country_ranges,
        "category_ranges": report.category_ranges
    }))
}

fn maxmind_geolite_config(
    config: &nvbes_core::config::AppConfig,
) -> anyhow::Result<nvbes_region::geo::MaxMindGeoLiteConfig> {
    Ok(nvbes_region::geo::MaxMindGeoLiteConfig {
        account_id: config
            .maxmind_account_id
            .clone()
            .ok_or_else(|| anyhow::anyhow!("NVBES_MAXMIND_ACCOUNT_ID is required"))?,
        license_key: config
            .maxmind_license_key
            .clone()
            .ok_or_else(|| anyhow::anyhow!("NVBES_MAXMIND_LICENSE_KEY is required"))?,
        eula_accepted: config.maxmind_geolite_eula_accepted,
    })
}
