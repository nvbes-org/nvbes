use std::net::IpAddr;

use super::{PersonalGeoDatabase, PersonalGeoDatabaseError, PersonalGeoRange};
use crate::geo::types::{GeoNetworkKind, GeoNetworkRelation};

#[test]
fn matches_cidr_range() {
    let range = PersonalGeoRange::new("203.0.113.0/24".parse().unwrap(), "FR").unwrap();
    let db = PersonalGeoDatabase::new(vec![range]);

    let location = db.lookup("203.0.113.42".parse().unwrap()).unwrap();
    assert_eq!(location.country_code, "FR");
    assert!(db.lookup("198.51.100.1".parse().unwrap()).is_none());
}

#[test]
fn rejects_unsupported_country_code() {
    let result = PersonalGeoRange::new("203.0.113.0/24".parse().unwrap(), "XX");
    assert_eq!(
        result,
        Err(PersonalGeoDatabaseError::UnsupportedCountryCode)
    );
}

#[test]
fn lookup_relation_includes_reputation_metadata() {
    let range = PersonalGeoRange::with_reputation(
        "203.0.113.0/24".parse().unwrap(),
        "DE",
        GeoNetworkKind::Datacenter,
        70,
        vec!["manual".to_string(), "datacenter".to_string()],
    )
    .unwrap();
    let db = PersonalGeoDatabase::new(vec![range]);
    let ip: IpAddr = "203.0.113.10".parse().unwrap();
    let (relation, location) = db.lookup_relation(ip).expect("match");
    assert_eq!(location.country_code, "DE");
    assert_eq!(relation.source_code, "personal_database");
    assert_eq!(relation.network.as_deref(), Some("203.0.113.0/24"));
    assert_eq!(relation.network_kind, Some(GeoNetworkKind::Datacenter));
    assert_eq!(relation.risk_score, Some(70));
}

#[test]
fn first_matching_range_wins_when_multiple_overlap() {
    let low = PersonalGeoRange::with_reputation(
        "203.0.113.0/24".parse().unwrap(),
        "FR",
        GeoNetworkKind::Residential,
        15,
        vec!["low".to_string()],
    )
    .unwrap();
    let high = PersonalGeoRange::with_reputation(
        "203.0.113.0/26".parse().unwrap(),
        "US",
        GeoNetworkKind::Datacenter,
        80,
        vec!["high".to_string()],
    )
    .unwrap();
    let db = PersonalGeoDatabase::new(vec![low, high]);
    let (relation, location) = db
        .lookup_relation("203.0.113.10".parse().unwrap())
        .expect("match");
    assert_eq!(location.country_code, "FR");
    assert_eq!(relation.risk_score, Some(15));
}

#[test]
fn default_new_range_uses_residential_reputation() {
    let range = PersonalGeoRange::new("198.51.100.0/24".parse().unwrap(), "US").unwrap();
    let (_relation, location): (GeoNetworkRelation, _) = PersonalGeoDatabase::new(vec![range])
        .lookup_relation("198.51.100.1".parse().unwrap())
        .expect("match");
    assert_eq!(location.country_code, "US");
}
