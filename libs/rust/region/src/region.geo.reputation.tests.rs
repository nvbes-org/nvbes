use super::{GeoReputation, score_relation};
use crate::geo::types::{GeoNetworkKind, GeoNetworkRelation};

fn relation(organization: &str) -> GeoNetworkRelation {
    GeoNetworkRelation {
        source_code: "arin".to_string(),
        registry: Some("arin".to_string()),
        network: Some("203.0.113.0/24".to_string()),
        start_ip: None,
        end_ip: None,
        asn: Some(64500),
        organization: Some(organization.to_string()),
        source_reference: None,
        network_kind: None,
        risk_score: None,
        risk_labels: Vec::new(),
    }
}

#[test]
fn scores_datacenter_and_vpn_higher_than_residential() {
    assert_eq!(
        score_relation(&relation("Amazon Web Services")).network_kind,
        GeoNetworkKind::Datacenter
    );
    assert_eq!(
        score_relation(&relation("Mullvad VPN")).network_kind,
        GeoNetworkKind::Vpn
    );
    assert!(
        score_relation(&relation("Mullvad VPN")).risk_score
            > score_relation(&relation("Example Broadband Telecom")).risk_score
    );
}

#[test]
fn scores_proxy_tor_mobile_and_unknown_networks() {
    assert_eq!(
        score_relation(&relation("Residential SOCKS proxy farm")).network_kind,
        GeoNetworkKind::Proxy
    );
    assert_eq!(score_relation(&relation(" tor-exit relay")).risk_score, 95);
    assert_eq!(
        score_relation(&relation("Carrier LTE 5G wireless")).network_kind,
        GeoNetworkKind::Mobile
    );
    assert_eq!(
        score_relation(&relation("Unclassified ASN holder")).network_kind,
        GeoNetworkKind::Unknown
    );
    assert_eq!(
        score_relation(&relation("Unclassified ASN holder")).risk_score,
        50
    );
}

#[test]
fn explicit_relation_labels_are_canonicalized() {
    let mut relation = relation("Example Provider");
    relation.network_kind = Some(GeoNetworkKind::Vpn);
    relation.risk_score = Some(90);
    relation.risk_labels = vec![
        "commercial_vpn".to_string(),
        "is_vpn".to_string(),
        "hosting".to_string(),
        "provider:cloudflare".to_string(),
        "source:loyalsoldier_geoip".to_string(),
    ];

    let reputation = score_relation(&relation);

    assert_eq!(
        reputation.risk_labels,
        vec![
            "vpn",
            "datacenter",
            "provider:cloudflare",
            "source:loyalsoldier_geoip"
        ]
    );
}

#[test]
fn unknown_explicit_relations_can_be_inferred_from_asn_text() {
    let mut relation = relation("Amazon Web Services");
    relation.network_kind = Some(GeoNetworkKind::Unknown);
    relation.risk_score = Some(35);
    relation.risk_labels = vec!["source:maxmind_geolite_asn".to_string()];

    let reputation = score_relation(&relation);

    assert_eq!(reputation.network_kind, GeoNetworkKind::Datacenter);
    assert_eq!(reputation.risk_score, 70);
    assert!(
        reputation
            .risk_labels
            .contains(&"source:maxmind_geolite_asn".to_string())
    );
}

#[test]
fn keeps_explicit_unknown_when_inference_is_not_stronger() {
    let mut relation = relation("Unclassified ASN holder");
    relation.network_kind = Some(GeoNetworkKind::Unknown);
    relation.risk_score = Some(50);
    relation.risk_labels = vec!["unknown".to_string(), "source:fixture".to_string()];

    let reputation = score_relation(&relation);
    assert_eq!(reputation.network_kind, GeoNetworkKind::Unknown);
    assert_eq!(reputation.risk_score, 50);
    assert!(reputation.risk_labels.contains(&"unknown".to_string()));
}

#[test]
fn constructors_expose_trusted_and_private_defaults() {
    let residential = GeoReputation::trusted_residential();
    assert_eq!(residential.network_kind, GeoNetworkKind::Residential);
    assert_eq!(residential.risk_score, 15);

    let private = GeoReputation::private_network();
    assert_eq!(private.network_kind, GeoNetworkKind::Unknown);
    assert_eq!(private.risk_score, 0);
    assert_eq!(private.risk_labels, vec!["private_network".to_string()]);
}

#[test]
fn returns_explicit_unknown_when_labels_are_already_meaningful() {
    let mut relation = relation("Unclassified ASN holder");
    relation.network_kind = Some(GeoNetworkKind::Unknown);
    relation.risk_score = Some(40);
    relation.risk_labels = vec!["residential".to_string()];

    let reputation = score_relation(&relation);
    assert_eq!(reputation.network_kind, GeoNetworkKind::Unknown);
    assert_eq!(reputation.risk_score, 40);
    assert!(reputation.risk_labels.contains(&"residential".to_string()));
}

#[test]
fn keeps_high_score_explicit_unknown_over_weaker_inference() {
    let mut relation = relation("Amazon Web Services");
    relation.network_kind = Some(GeoNetworkKind::Unknown);
    relation.risk_score = Some(90);
    relation.risk_labels = vec!["source:fixture".to_string()];

    let reputation = score_relation(&relation);
    assert_eq!(reputation.network_kind, GeoNetworkKind::Unknown);
    assert_eq!(reputation.risk_score, 90);
}

#[test]
fn explicit_empty_labels_receive_kind_fallback() {
    let mut relation = relation("Example Provider");
    relation.network_kind = Some(GeoNetworkKind::Mobile);
    relation.risk_score = Some(35);
    relation.risk_labels = Vec::new();

    let reputation = score_relation(&relation);
    assert_eq!(reputation.network_kind, GeoNetworkKind::Mobile);
    assert!(reputation.risk_labels.contains(&"mobile".to_string()));
}

#[test]
fn partial_explicit_fields_fall_back_to_text_classification() {
    let mut kind_only = relation(" tor exit node");
    kind_only.network_kind = Some(GeoNetworkKind::Unknown);
    kind_only.risk_score = None;
    assert_eq!(score_relation(&kind_only).network_kind, GeoNetworkKind::Tor);

    let mut score_only = relation("openvpn gateway");
    score_only.network_kind = None;
    score_only.risk_score = Some(10);
    assert_eq!(
        score_relation(&score_only).network_kind,
        GeoNetworkKind::Vpn
    );
}

#[test]
fn classifies_from_source_reference_when_organization_missing() {
    let mut relation = relation("ignored");
    relation.organization = None;
    relation.source_reference = Some("bright data proxy pool".to_string());
    assert_eq!(
        score_relation(&relation).network_kind,
        GeoNetworkKind::Proxy
    );
}
