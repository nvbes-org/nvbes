#[path = "region.geo.database.rs"]
pub mod database;
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
#[path = "region.geo.resolver.rs"]
pub mod resolver;
#[path = "region.geo.types.rs"]
pub mod types;

pub use database::{PersonalGeoDatabase, PersonalGeoDatabaseError, PersonalGeoRange};
pub use ip::{is_private_or_special_ip, parse_ip};
pub use maintenance::{GeoMaintenanceReport, run_geo_maintenance, run_geo_maintenance_tx};
pub use persistence::{
    GeoLookupRecordContext, cached_remote_lookup_tx, load_personal_geo_database_tx,
    record_geo_resolution_tx, resolve_cached_geo_tx,
};
pub use personal::{
    PersonalGeoRangeView, PersonalGeoStoreError, UpsertPersonalGeoRangeInput,
    disable_personal_geo_range, disable_personal_geo_range_tx, list_personal_geo_ranges,
    upsert_personal_geo_range, upsert_personal_geo_range_tx,
};
pub use rdap::{DEFAULT_RDAP_REGISTRIES, RdapClient, RdapLookup, RdapLookupError, RdapRegistry};
pub use resolver::{GeoLookupRequest, GeoResolver};
pub use types::{GeoConfidence, GeoEvidence, GeoEvidenceSource, GeoLocation, GeoResolution};
