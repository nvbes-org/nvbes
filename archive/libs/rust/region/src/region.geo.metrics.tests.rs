use std::time::Duration;

use super::{
    record_geo_cache_lookup, record_geo_resolution, record_ip_intelligence_lookup,
    record_rdap_lookup,
};
use crate::geo::types::{GeoConfidence, GeoEvidenceSource, GeoResolution};

#[test]
fn metrics_recorders_accept_local_outcomes() {
    let resolution = GeoResolution::unresolved(None, false, Vec::new());
    record_geo_resolution(&resolution);
    record_geo_cache_lookup("hit", Duration::from_millis(5));
    record_rdap_lookup("ripe", "hit", Duration::from_millis(3));
    record_ip_intelligence_lookup("test_provider", "hit", Duration::from_millis(2));

    let resolved = GeoResolution {
        location: crate::geo::types::GeoLocation::from_country_code("FR"),
        confidence: GeoConfidence::High,
        source: GeoEvidenceSource::PersonalDatabase,
        ip: None,
        private_network: false,
        network_kind: crate::geo::types::GeoNetworkKind::Residential,
        risk_score: 15,
        risk_labels: vec!["personal_database".to_string()],
        evidence: Vec::new(),
    };
    record_geo_resolution(&resolved);
}
