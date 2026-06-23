#[path = "region.geo.cache.rs"]
pub mod cache;
#[path = "region.geo.database.rs"]
pub mod database;
#[path = "region.geo.intelligence.rs"]
pub mod intelligence;
#[path = "region.geo.ip.rs"]
pub mod ip;
#[path = "region.geo.maintenance.rs"]
pub mod maintenance;
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

pub use cache::{
    cached_ip_intelligence_tx, cached_remote_lookup_tx, merge_cached_intelligence,
    resolve_cached_geo_tx,
};
pub use database::{PersonalGeoDatabase, PersonalGeoDatabaseError, PersonalGeoRange};
pub use intelligence::{
    IpIntelligenceInput, IpIntelligenceLookup, cache_ip_intelligence_tx,
    merge_intelligence_into_relation, normalize_ip_intelligence,
};
pub use ip::{is_private_or_special_ip, parse_ip};
pub use maintenance::{GeoMaintenanceReport, run_geo_maintenance, run_geo_maintenance_tx};
pub use persistence::{
    GeoLookupRecordContext, load_personal_geo_database_tx, record_geo_resolution_tx,
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
