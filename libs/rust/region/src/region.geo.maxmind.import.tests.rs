use super::relation_key;
use crate::geo::maxmind_types::MaxMindGeoLiteRange;

#[test]
fn relation_key_includes_source_and_network() {
    let range = MaxMindGeoLiteRange {
        source_code: "maxmind_geolite_country_csv",
        edition_id: "GeoLite2-Country-CSV",
        network: "203.0.113.0/24".parse().unwrap(),
        country_code: Some("FR".into()),
        geoname_id: None,
        asn: None,
        organization: None,
    };
    assert_eq!(
        relation_key(&range),
        "maxmind_geolite_country_csv:203.0.113.0/24"
    );
}
