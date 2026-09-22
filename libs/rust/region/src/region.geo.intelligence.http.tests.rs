use super::{
    IpIntelligenceHttpProvider, normalize_http_body, provider_from_spec, providers_from_specs,
};
use crate::geo::types::GeoNetworkKind;

#[test]
fn provider_url_replaces_ip_placeholder() {
    let provider = IpIntelligenceHttpProvider {
        source_code: "fixture".to_string(),
        url_template: "https://example.test/ip/{ip}".to_string(),
        authorization_header: None,
    };

    assert_eq!(
        provider.url_for_ip("8.8.8.8".parse().unwrap()),
        "https://example.test/ip/8.8.8.8"
    );
}

#[test]
fn parses_provider_specs() {
    let providers = providers_from_specs(&[
        "ipinfo|https://example.test/{ip}|Bearer token".to_string(),
        "maxmind|https://risk.example/{ip}".to_string(),
    ])
    .unwrap();

    assert_eq!(providers.len(), 2);
    assert_eq!(providers[0].source_code, "ipinfo");
    assert_eq!(
        providers[0].authorization_header.as_deref(),
        Some("Bearer token")
    );
    assert!(provider_from_spec("broken").is_err());
    assert!(provider_from_spec("source|https://example.test/no-placeholder").is_err());
}

#[test]
fn normalizes_common_privacy_payload() {
    let body = serde_json::json!({
        "country": "fr",
        "asn": "AS64500",
        "org": "Example Network",
        "privacy": {"vpn": true, "proxy": false, "tor": false, "hosting": false},
        "risk_labels": ["commercial_vpn"]
    });

    let lookup =
        normalize_http_body("fixture_provider", "8.8.8.8".parse().unwrap(), &body).unwrap();

    assert_eq!(lookup.country_code.as_deref(), Some("FR"));
    assert_eq!(lookup.relation.asn, Some(64500));
    assert_eq!(lookup.relation.network_kind, Some(GeoNetworkKind::Vpn));
    assert_eq!(lookup.relation.risk_score, Some(90));
    assert!(lookup.relation.risk_labels.contains(&"vpn".to_string()));
    assert!(
        !lookup
            .relation
            .risk_labels
            .contains(&"commercial_vpn".to_string())
    );
}

#[tokio::test]
async fn http_client_fetches_provider_payload() {
    use super::IpIntelligenceHttpClient;
    use reqwest::Client;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ip/8.8.8.8"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "country": "US",
            "privacy": {"hosting": true}
        })))
        .mount(&server)
        .await;

    let client = IpIntelligenceHttpClient::new(
        Client::new(),
        vec![super::IpIntelligenceHttpProvider {
            source_code: "fixture".to_string(),
            url_template: format!("{}/ip/{{ip}}", server.uri()),
            authorization_header: None,
        }],
    );

    let lookup = client
        .lookup("8.8.8.8".parse().unwrap())
        .await
        .expect("lookup")
        .expect("payload");
    assert_eq!(lookup.country_code.as_deref(), Some("US"));
    assert_eq!(
        lookup.relation.network_kind,
        Some(GeoNetworkKind::Datacenter)
    );
}

#[test]
fn normalize_http_body_rejects_empty_payload() {
    let body = serde_json::json!({"note": "no signals"});
    assert!(super::normalize_http_body("fixture", "1.1.1.1".parse().unwrap(), &body).is_err());
}

#[tokio::test]
async fn http_client_skips_http_misses() {
    use reqwest::Client;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ip/1.1.1.1"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let client = super::IpIntelligenceHttpClient::new(
        Client::new(),
        vec![super::IpIntelligenceHttpProvider {
            source_code: "miss".to_string(),
            url_template: format!("{}/ip/{{ip}}", server.uri()),
            authorization_header: None,
        }],
    );

    assert!(
        client
            .lookup("1.1.1.1".parse().unwrap())
            .await
            .expect("lookup")
            .is_none()
    );
}

#[tokio::test]
async fn http_client_surfaces_json_decode_errors() {
    use reqwest::Client;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ip/8.8.8.8"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not-json"))
        .mount(&server)
        .await;

    let client = super::IpIntelligenceHttpClient::new(
        Client::new(),
        vec![super::IpIntelligenceHttpProvider {
            source_code: "bad_json".to_string(),
            url_template: format!("{}/ip/{{ip}}", server.uri()),
            authorization_header: None,
        }],
    );

    assert!(client.lookup("8.8.8.8".parse().unwrap()).await.is_err());
}
