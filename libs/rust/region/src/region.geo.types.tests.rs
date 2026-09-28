use std::net::IpAddr;

use super::{
    GeoConfidence, GeoEvidence, GeoEvidenceSource, GeoLocation, GeoNetworkKind, GeoNetworkRelation,
    GeoResolution, GeoRiskSignal, canonical_geo_risk_label, canonicalize_geo_risk_labels,
    push_unique_label,
};

#[test]
fn enum_labels_use_stable_database_values() {
    assert_eq!(GeoConfidence::High.as_str(), "high");
    assert_eq!(
        GeoEvidenceSource::PersonalDatabase.as_str(),
        "personal_database"
    );
    assert_eq!(GeoNetworkKind::Datacenter.as_str(), "datacenter");
    assert_eq!(GeoRiskSignal::Tor.as_str(), "tor");
}

#[test]
fn network_kind_from_label_maps_known_values() {
    assert_eq!(GeoNetworkKind::from_label("vpn"), GeoNetworkKind::Vpn);
    assert_eq!(
        GeoNetworkKind::from_label("unknown-label"),
        GeoNetworkKind::Unknown
    );
}

#[test]
fn canonical_geo_risk_label_normalizes_aliases() {
    let cases = [
        ("commercial_vpn", Some("vpn")),
        ("is_vpn", Some("vpn")),
        ("hosting", Some("datacenter")),
        ("mobile_carrier", Some("mobile")),
        ("consumer_isp", Some("residential")),
        ("source:ipinfo", None),
        ("not-a-signal", None),
    ];
    for (input, expected) in cases {
        assert_eq!(canonical_geo_risk_label(input), expected, "input={input}");
    }
}

#[test]
fn canonicalize_geo_risk_labels_preserves_namespaced_labels() {
    let labels = canonicalize_geo_risk_labels(vec![
        "commercial_vpn".to_string(),
        "source:test_provider".to_string(),
        "provider:cloudflare".to_string(),
        "category:tor".to_string(),
        "hosting".to_string(),
    ]);
    assert!(labels.contains(&"vpn".to_string()));
    assert!(labels.contains(&"datacenter".to_string()));
    assert!(labels.contains(&"source:test_provider".to_string()));
    assert!(labels.contains(&"provider:cloudflare".to_string()));
    assert!(labels.contains(&"category:tor".to_string()));
    assert_eq!(
        labels
            .iter()
            .filter(|label| label.as_str() == "vpn")
            .count(),
        1
    );
}

#[test]
fn push_unique_label_deduplicates_case_sensitive_entries() {
    let mut labels = Vec::new();
    push_unique_label(&mut labels, "vpn");
    push_unique_label(&mut labels, "vpn");
    push_unique_label(&mut labels, "tor");
    assert_eq!(labels, vec!["vpn".to_string(), "tor".to_string()]);
}

#[test]
fn geo_evidence_constructors_set_expected_fields() {
    let location = GeoLocation::from_country_code("FR").expect("FR profile");
    let relation = GeoNetworkRelation {
        source_code: "personal_database".to_string(),
        registry: None,
        network: Some("203.0.113.0/24".to_string()),
        start_ip: None,
        end_ip: None,
        asn: None,
        organization: None,
        source_reference: None,
        network_kind: Some(GeoNetworkKind::Residential),
        risk_score: Some(15),
        risk_labels: vec!["personal_database".to_string()],
    };

    let accepted = GeoEvidence::accepted(
        GeoEvidenceSource::PersonalDatabase,
        &location,
        GeoConfidence::High,
        "matched personal range",
    );
    assert!(accepted.accepted);
    assert_eq!(accepted.country_code.as_deref(), Some("FR"));

    let with_relation = GeoEvidence::accepted_with_relation(
        GeoEvidenceSource::RemoteLookup,
        &location,
        GeoConfidence::Medium,
        "rdap hit",
        relation,
    );
    assert!(with_relation.relation.is_some());

    let rejected = GeoEvidence::rejected(GeoEvidenceSource::Fallback, Some("xx"), "unsupported");
    assert!(!rejected.accepted);
    assert_eq!(rejected.country_code.as_deref(), Some("XX"));
}

#[test]
fn geo_resolution_unresolved_marks_private_and_public_unknown() {
    let private = GeoResolution::unresolved(
        Some("10.0.0.1".parse::<IpAddr>().unwrap()),
        true,
        Vec::new(),
    );
    assert!(private.private_network);
    assert_eq!(private.risk_score, 0);
    assert!(private.risk_labels.contains(&"private_network".to_string()));

    let unknown = GeoResolution::unresolved(None, false, Vec::new());
    assert!(!unknown.private_network);
    assert_eq!(unknown.risk_score, 50);
    assert!(unknown.risk_labels.contains(&"unknown".to_string()));
}
