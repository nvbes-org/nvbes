use super::{
    DEFAULT_RDAP_REGISTRIES, RdapRegistry, first_entity_name, first_notice_country, parse_asn,
    string_field,
};
use crate::geo::types::GeoLocation;

#[test]
fn parses_asn_handle() {
    assert_eq!(parse_asn("AS13335".to_string()), Some(13335));
    assert_eq!(parse_asn("NET-8-8-8-0-1".to_string()), None);
}

#[test]
fn extracts_first_entity_handle() {
    let body = serde_json::json!({"entities": [{"handle": "ORG-EXAMPLE"}]});
    assert_eq!(first_entity_name(&body).as_deref(), Some("ORG-EXAMPLE"));
}

#[test]
fn rdap_registry_urls_and_codes_are_stable() {
    for registry in DEFAULT_RDAP_REGISTRIES {
        assert!(!registry.code().is_empty());
        assert!(registry.base_url().starts_with("https://"));
    }
    assert_eq!(RdapRegistry::Ripe.code(), "ripe");
    assert!(RdapRegistry::Apnic.base_url().contains("apnic"));
}

#[test]
fn string_field_trims_values() {
    let body = serde_json::json!({"country": "  FR  "});
    assert_eq!(string_field(&body, "country").as_deref(), Some("FR"));
    assert!(string_field(&body, "missing").is_none());
}

#[test]
fn first_notice_country_reads_nested_notices() {
    let body = serde_json::json!({
        "notices": [{"country": "US"}, {"country": "  CA  "}]
    });
    assert_eq!(first_notice_country(&body).as_deref(), Some("US"));
}

#[test]
fn rdap_country_fields_map_to_supported_locations() {
    let body = serde_json::json!({"country": "de"});
    let country = string_field(&body, "country").expect("country");
    assert!(GeoLocation::from_country_code(&country).is_some());
}
