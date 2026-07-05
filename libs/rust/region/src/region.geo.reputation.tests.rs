use super::score_relation;
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
