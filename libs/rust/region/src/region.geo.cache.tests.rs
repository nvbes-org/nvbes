use super::merge_cached_intelligence;
use crate::geo::intelligence::{IpIntelligenceLookup, normalize_ip_intelligence};
use crate::geo::types::{GeoNetworkKind, GeoNetworkRelation};

#[test]
fn merge_cached_intelligence_returns_original_without_lookup() {
    let relation = GeoNetworkRelation {
        source_code: "ripe".to_string(),
        registry: Some("ripe".to_string()),
        network: Some("203.0.113.0/24".to_string()),
        start_ip: None,
        end_ip: None,
        asn: None,
        organization: None,
        source_reference: None,
        network_kind: Some(GeoNetworkKind::Unknown),
        risk_score: Some(30),
        risk_labels: vec!["source:ripe".to_string()],
    };
    let merged = merge_cached_intelligence(relation.clone(), None);
    assert_eq!(merged, relation);
}

#[test]
fn merge_cached_intelligence_prefers_higher_risk_intelligence() {
    let relation = GeoNetworkRelation {
        source_code: "ripe".to_string(),
        registry: Some("ripe".to_string()),
        network: Some("203.0.113.0/24".to_string()),
        start_ip: None,
        end_ip: None,
        asn: None,
        organization: None,
        source_reference: None,
        network_kind: Some(GeoNetworkKind::Unknown),
        risk_score: Some(30),
        risk_labels: vec!["source:ripe".to_string()],
    };
    let intelligence = IpIntelligenceLookup {
        country_code: Some("FR".to_string()),
        relation: normalize_ip_intelligence(crate::geo::intelligence::IpIntelligenceInput {
            source_code: "test_provider".to_string(),
            ip: "203.0.113.42".parse().unwrap(),
            country_code: Some("FR".to_string()),
            asn: None,
            organization: None,
            network: Some("203.0.113.0/24".to_string()),
            source_reference: None,
            is_vpn: true,
            is_proxy: false,
            is_tor: false,
            is_datacenter: false,
            is_mobile: false,
            is_residential: false,
            risk_score: None,
            risk_labels: Vec::new(),
        })
        .relation,
    };
    let merged = merge_cached_intelligence(relation, Some(&intelligence));
    assert_eq!(merged.network_kind, Some(GeoNetworkKind::Vpn));
    assert_eq!(merged.risk_score, Some(90));
}
