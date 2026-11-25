use ipnet::IpNet;

use super::{GeoLookupRequest, GeoResolver};
use crate::geo::{
    database::{PersonalGeoDatabase, PersonalGeoRange},
    types::{GeoConfidence, GeoEvidenceSource},
};

#[test]
fn trusted_header_wins_before_database() {
    let db = PersonalGeoDatabase::new(vec![
        PersonalGeoRange::new("8.8.8.0/24".parse::<IpNet>().unwrap(), "US").unwrap(),
    ]);
    let resolver = GeoResolver::new(db);

    let result = resolver.resolve(GeoLookupRequest {
        ip: Some("8.8.8.8".parse().unwrap()),
        trusted_country_header: Some("FR"),
        ..GeoLookupRequest::default()
    });

    assert_eq!(result.location.unwrap().country_code, "FR");
    assert_eq!(result.source, GeoEvidenceSource::TrustedProxyHeader);
}

#[test]
fn database_wins_before_provider_header() {
    let db = PersonalGeoDatabase::new(vec![
        PersonalGeoRange::new("8.8.8.0/24".parse::<IpNet>().unwrap(), "US").unwrap(),
    ]);
    let resolver = GeoResolver::new(db);

    let result = resolver.resolve(GeoLookupRequest {
        ip: Some("8.8.8.8".parse().unwrap()),
        provider_country_header: Some("FR"),
        ..GeoLookupRequest::default()
    });

    assert_eq!(result.location.unwrap().country_code, "US");
    assert_eq!(result.source, GeoEvidenceSource::PersonalDatabase);
}

#[test]
fn private_ip_can_only_use_non_ip_fallback() {
    let resolver = GeoResolver::default();

    let result = resolver.resolve(GeoLookupRequest {
        ip: Some("10.0.0.8".parse().unwrap()),
        stored_profile_country: Some("CH"),
        ..GeoLookupRequest::default()
    });

    assert_eq!(result.location.unwrap().country_code, "CH");
    assert_eq!(result.confidence, GeoConfidence::Low);
    assert!(result.private_network);
}

#[test]
fn trusted_country_can_still_use_network_intelligence_risk() {
    let resolver = GeoResolver::default();
    let intelligence = crate::geo::normalize_ip_intelligence(crate::geo::IpIntelligenceInput {
        source_code: "fixture".to_string(),
        ip: "8.8.8.8".parse().unwrap(),
        country_code: None,
        asn: Some(64500),
        organization: Some("Example VPN".to_string()),
        network: Some("8.8.8.0/24".to_string()),
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

    let result = resolver.resolve(GeoLookupRequest {
        ip: Some("8.8.8.8".parse().unwrap()),
        trusted_country_header: Some("FR"),
        network_intelligence: Some(&intelligence),
        ..GeoLookupRequest::default()
    });

    assert_eq!(result.location.unwrap().country_code, "FR");
    assert_eq!(result.source, GeoEvidenceSource::TrustedProxyHeader);
    assert_eq!(result.network_kind, crate::geo::GeoNetworkKind::Vpn);
    assert_eq!(result.risk_score, 90);
}
