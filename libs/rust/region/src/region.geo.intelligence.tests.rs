use super::{IpIntelligenceInput, network_relation_key, normalize_ip_intelligence};
use crate::geo::types::GeoNetworkKind;

#[test]
fn normalizes_vpn_signal_with_default_score() {
    let lookup = normalize_ip_intelligence(IpIntelligenceInput {
        source_code: "test_provider".to_string(),
        ip: "203.0.113.42".parse().unwrap(),
        country_code: Some("fr".to_string()),
        asn: Some(64500),
        organization: Some("Example VPN".to_string()),
        network: Some("203.0.113.0/24".to_string()),
        source_reference: Some("fixture".to_string()),
        is_vpn: true,
        is_proxy: false,
        is_tor: false,
        is_datacenter: false,
        is_mobile: false,
        is_residential: false,
        risk_score: None,
        risk_labels: Vec::new(),
    });

    assert_eq!(lookup.country_code.as_deref(), Some("FR"));
    assert_eq!(lookup.relation.source_code, "ip_intelligence");
    assert_eq!(lookup.relation.network_kind, Some(GeoNetworkKind::Vpn));
    assert_eq!(lookup.relation.risk_score, Some(90));
    assert!(lookup.relation.risk_labels.contains(&"vpn".to_string()));
    assert!(
        lookup
            .relation
            .risk_labels
            .contains(&"source:test_provider".to_string())
    );
}

#[test]
fn relation_key_includes_provider_for_ip_intelligence() {
    let lookup = normalize_ip_intelligence(IpIntelligenceInput {
        source_code: "test_provider".to_string(),
        ip: "8.8.8.8".parse().unwrap(),
        country_code: None,
        asn: None,
        organization: None,
        network: Some("8.8.8.0/24".to_string()),
        source_reference: Some("fixture".to_string()),
        is_vpn: false,
        is_proxy: false,
        is_tor: false,
        is_datacenter: true,
        is_mobile: false,
        is_residential: false,
        risk_score: None,
        risk_labels: Vec::new(),
    });

    assert_eq!(
        network_relation_key(&lookup.relation),
        "provider:test_provider:network:8.8.8.0/24"
    );
}

#[test]
fn canonicalizes_provider_labels() {
    let lookup = normalize_ip_intelligence(IpIntelligenceInput {
        source_code: "test_provider".to_string(),
        ip: "203.0.113.42".parse().unwrap(),
        country_code: None,
        asn: None,
        organization: None,
        network: None,
        source_reference: None,
        is_vpn: true,
        is_proxy: false,
        is_tor: false,
        is_datacenter: false,
        is_mobile: false,
        is_residential: false,
        risk_score: None,
        risk_labels: vec![
            "commercial_vpn".to_string(),
            "is_vpn".to_string(),
            "hosting".to_string(),
            "source:test_provider".to_string(),
        ],
    });

    assert!(lookup.relation.risk_labels.contains(&"vpn".to_string()));
    assert!(
        lookup
            .relation
            .risk_labels
            .contains(&"datacenter".to_string())
    );
    assert!(
        !lookup
            .relation
            .risk_labels
            .contains(&"commercial_vpn".to_string())
    );
    assert_eq!(
        lookup
            .relation
            .risk_labels
            .iter()
            .filter(|label| label.as_str() == "vpn")
            .count(),
        1
    );
}
