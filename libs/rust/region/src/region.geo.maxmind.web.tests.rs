use serde_json::json;

use super::{
    MaxMindGeoLiteWebLookup, maxmind_ip_relation, maxmind_web_evidence_source, relation_key,
};
use crate::geo::types::{GeoEvidenceSource, GeoNetworkRelation};

#[test]
fn maps_country_response_to_relation() {
    let body = json!({"country": {"iso_code": "FR"}});
    let (relation, location) =
        maxmind_ip_relation("203.0.113.7".parse().unwrap(), &body).expect("relation should parse");

    assert_eq!(location.country_code, "FR");
    assert_eq!(relation.network.as_deref(), Some("203.0.113.7/32"));
}

#[test]
fn relation_key_uses_network_when_present() {
    let relation = GeoNetworkRelation {
        source_code: "maxmind_geolite_web".to_string(),
        registry: Some("maxmind".to_string()),
        network: Some("203.0.113.7/32".to_string()),
        start_ip: None,
        end_ip: None,
        asn: None,
        organization: None,
        source_reference: None,
        network_kind: None,
        risk_score: None,
        risk_labels: Vec::new(),
    };
    assert_eq!(
        relation_key(&relation),
        "maxmind_geolite_web:203.0.113.7/32"
    );
}

#[test]
fn relation_key_falls_back_to_source_code_without_network() {
    let relation = GeoNetworkRelation {
        source_code: "maxmind_geolite_web".to_string(),
        registry: None,
        network: None,
        start_ip: None,
        end_ip: None,
        asn: None,
        organization: None,
        source_reference: None,
        network_kind: None,
        risk_score: None,
        risk_labels: Vec::new(),
    };
    assert_eq!(relation_key(&relation), "maxmind_geolite_web");
}

#[test]
fn web_lookup_converts_to_rdap_lookup_and_evidence_source() {
    let body = json!({"country": {"iso_code": "FR"}});
    let (relation, location) = maxmind_ip_relation("203.0.113.7".parse().unwrap(), &body).unwrap();
    let lookup = MaxMindGeoLiteWebLookup { location, relation };
    let rdap = lookup.as_rdap_lookup();
    assert_eq!(rdap.location.country_code, "FR");
    assert_eq!(
        maxmind_web_evidence_source(),
        GeoEvidenceSource::RemoteLookup
    );
}
