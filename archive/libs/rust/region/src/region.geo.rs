#[path = "region.geo.cache.rs"]
pub mod cache;
#[path = "region.geo.database.rs"]
pub mod database;
#[path = "region.geo.intelligence.rs"]
pub mod intelligence;
#[path = "region.geo.intelligence.http.rs"]
pub mod intelligence_http;
#[path = "region.geo.ip.rs"]
pub mod ip;
#[path = "region.geo.loyalsoldier.download.rs"]
mod loyalsoldier_download;
#[path = "region.geo.loyalsoldier.import.rs"]
pub mod loyalsoldier_import;
#[path = "region.geo.loyalsoldier.types.rs"]
mod loyalsoldier_types;
#[path = "region.geo.maintenance.rs"]
pub mod maintenance;
#[path = "region.geo.maxmind.csv.rs"]
mod maxmind_csv;
#[path = "region.geo.maxmind.download.rs"]
mod maxmind_download;
#[path = "region.geo.maxmind.import.rs"]
pub mod maxmind_import;
#[path = "region.geo.maxmind.types.rs"]
mod maxmind_types;
#[path = "region.geo.maxmind.web.rs"]
pub mod maxmind_web;
#[path = "region.geo.metrics.rs"]
mod metrics;
#[path = "region.geo.persistence.rs"]
pub mod persistence;
#[path = "region.geo.personal.rs"]
pub mod personal;
#[path = "region.geo.rdap.rs"]
pub mod rdap;
#[path = "region.geo.reputation.rs"]
pub mod reputation;
#[path = "region.geo.resolver.rs"]
pub mod resolver;
#[path = "region.geo.types.rs"]
pub mod types;
#[path = "region.geo.v2fly.dat.rs"]
mod v2fly_dat;
#[path = "region.geo.v2fly.import.rs"]
pub mod v2fly_import;

#[cfg(test)]
#[path = "region.geo.property.tests.rs"]
mod property_tests;

pub use cache::{
    cached_ip_intelligence_tx, cached_remote_lookup_tx, merge_cached_intelligence,
    resolve_cached_geo_tx,
};
pub use database::{PersonalGeoDatabase, PersonalGeoDatabaseError, PersonalGeoRange};
pub use intelligence::{
    IpIntelligenceInput, IpIntelligenceLookup, cache_ip_intelligence_tx,
    merge_intelligence_into_relation, normalize_ip_intelligence,
};
pub use intelligence_http::{
    IpIntelligenceHttpClient, IpIntelligenceHttpError, IpIntelligenceHttpProvider,
    IpIntelligenceProviderSpecError, normalize_http_body, provider_from_spec, providers_from_specs,
};
pub use ip::{is_private_or_special_ip, parse_ip};
pub use loyalsoldier_import::{
    LoyalsoldierGeoIpImportError, import_latest_loyalsoldier_geoip,
    run_scheduled_loyalsoldier_geoip_import,
};
pub use loyalsoldier_types::{LOYALSOLDIER_GEOIP_SOURCE_CODE, LoyalsoldierGeoIpImportReport};
pub use maintenance::{GeoMaintenanceReport, run_geo_maintenance, run_geo_maintenance_tx};
pub use maxmind_import::{
    MaxMindGeoLiteImportError, MaxMindGeoLiteImportReport, import_latest_maxmind_geolite,
    run_scheduled_maxmind_geolite_import,
};
pub use maxmind_types::{
    MAXMIND_GEOLITE_ASN_SOURCE_CODE, MAXMIND_GEOLITE_CITY_SOURCE_CODE,
    MAXMIND_GEOLITE_COUNTRY_SOURCE_CODE, MAXMIND_GEOLITE_SOURCE_CODES,
    MAXMIND_GEOLITE_WEB_SOURCE_CODE, MaxMindGeoLiteConfig, MaxMindGeoLiteConfigError,
};
pub use maxmind_web::{
    MaxMindGeoLiteWebClient, MaxMindGeoLiteWebError, MaxMindGeoLiteWebLookup,
    cache_maxmind_geolite_web_tx,
};
pub use persistence::{
    GeoLookupPurpose, GeoLookupRecordContext, load_personal_geo_database_tx,
    record_geo_resolution_tx,
};
pub use personal::{
    PersonalGeoRangeView, PersonalGeoStoreError, UpsertPersonalGeoRangeInput,
    disable_personal_geo_range, disable_personal_geo_range_tx, list_personal_geo_ranges,
    upsert_personal_geo_range, upsert_personal_geo_range_tx,
};
pub use rdap::{DEFAULT_RDAP_REGISTRIES, RdapClient, RdapLookup, RdapLookupError, RdapRegistry};
pub use reputation::{GeoReputation, score_relation};
pub use resolver::{GeoLookupRequest, GeoResolver};
pub use types::{
    GeoConfidence, GeoEvidence, GeoEvidenceSource, GeoLocation, GeoNetworkKind, GeoResolution,
    GeoRiskSignal,
};
pub use v2fly_import::{
    V2FLY_GEOIP_SOURCE_CODE, V2flyGeoIpImportError, V2flyGeoIpImportReport,
    import_latest_v2fly_geoip, run_scheduled_v2fly_geoip_import,
};
