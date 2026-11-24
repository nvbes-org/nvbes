use super::{
    DEFAULT_RDAP_REGISTRIES, RdapClient, RdapRegistry, first_entity_name, first_notice_country,
    parse_asn, string_field,
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

#[tokio::test]
async fn lookup_with_base_parses_successful_rdap_payload() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/registry/ip/93.184.216.34"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "country": "US",
            "startAddress": "93.184.216.0",
            "endAddress": "93.184.216.255",
            "handle": "AS15133",
            "name": "Example ISP"
        })))
        .mount(&server)
        .await;

    let client = RdapClient::new(reqwest::Client::new());
    let base = format!("{}/registry/ip", server.uri());
    let lookup = client
        .lookup_with_base("ripe", &base, "93.184.216.34".parse().unwrap())
        .await
        .expect("lookup")
        .expect("hit");
    assert_eq!(lookup.location.country_code, "US");
    assert_eq!(lookup.relation.asn, Some(15133));
}

#[tokio::test]
async fn lookup_with_base_treats_http_miss_as_none() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/registry/ip/93.184.216.34"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let client = RdapClient::new(reqwest::Client::new());
    let base = format!("{}/registry/ip", server.uri());
    let lookup = client
        .lookup_with_base("ripe", &base, "93.184.216.34".parse().unwrap())
        .await
        .expect("lookup");
    assert!(lookup.is_none());
}

#[tokio::test]
async fn lookup_with_base_skips_responses_without_country() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/registry/ip/93.184.216.34"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "startAddress": "93.184.216.0",
            "endAddress": "93.184.216.255"
        })))
        .mount(&server)
        .await;

    let client = RdapClient::new(reqwest::Client::new());
    let base = format!("{}/registry/ip", server.uri());
    let lookup = client
        .lookup_with_base("ripe", &base, "93.184.216.34".parse().unwrap())
        .await
        .expect("lookup");
    assert!(lookup.is_none());
}

#[tokio::test]
async fn lookup_with_base_surfaces_json_decode_errors() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/registry/ip/93.184.216.34"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not-json"))
        .mount(&server)
        .await;

    let client = RdapClient::new(reqwest::Client::new());
    let base = format!("{}/registry/ip", server.uri());
    let error = client
        .lookup_with_base("ripe", &base, "93.184.216.34".parse().unwrap())
        .await
        .expect_err("decode error");
    assert!(matches!(error, super::RdapLookupError::Request(_)));
}

#[tokio::test]
async fn lookup_returns_none_when_no_registry_matches() {
    let client = RdapClient::with_registries(reqwest::Client::new(), vec![]);
    let lookup = client
        .lookup("93.184.216.34".parse().unwrap())
        .await
        .expect("lookup");
    assert!(lookup.is_none());
}
