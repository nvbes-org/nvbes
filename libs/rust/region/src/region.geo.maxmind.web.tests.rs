use chrono::{Duration, Utc};
use serde_json::json;
use sqlx::PgPool;

use super::{
    MaxMindGeoLiteWebLookup, cache_maxmind_geolite_web_tx, maxmind_ip_relation,
    maxmind_web_evidence_source, relation_key,
};
use crate::geo::types::{GeoEvidenceSource, GeoNetworkRelation};

#[test]
fn maps_country_response_to_relation() {
    let body = json!({"country": {"iso_code": "FR"}});
    let (relation, location) =
        maxmind_ip_relation("203.0.113.7".parse().unwrap(), &body).expect("relation should parse");

    assert_eq!(location.country_code, "FR");
    assert_eq!(relation.network.as_deref(), Some("203.0.113.7/32"));
}

#[test]
fn relation_key_uses_network_when_present() {
    let relation = GeoNetworkRelation {
        source_code: "maxmind_geolite_web".to_string(),
        registry: Some("maxmind".to_string()),
        network: Some("203.0.113.7/32".to_string()),
        start_ip: None,
        end_ip: None,
        asn: None,
        organization: None,
        source_reference: None,
        network_kind: None,
        risk_score: None,
        risk_labels: Vec::new(),
    };
    assert_eq!(
        relation_key(&relation),
        "maxmind_geolite_web:203.0.113.7/32"
    );
}

#[test]
fn relation_key_falls_back_to_source_code_without_network() {
    let relation = GeoNetworkRelation {
        source_code: "maxmind_geolite_web".to_string(),
        registry: None,
        network: None,
        start_ip: None,
        end_ip: None,
        asn: None,
        organization: None,
        source_reference: None,
        network_kind: None,
        risk_score: None,
        risk_labels: Vec::new(),
    };
    assert_eq!(relation_key(&relation), "maxmind_geolite_web");
}

#[test]
fn web_lookup_converts_to_rdap_lookup_and_evidence_source() {
    let body = json!({"country": {"iso_code": "FR"}});
    let (relation, location) = maxmind_ip_relation("203.0.113.7".parse().unwrap(), &body).unwrap();
    let lookup = MaxMindGeoLiteWebLookup { location, relation };
    let rdap = lookup.as_rdap_lookup();
    assert_eq!(rdap.location.country_code, "FR");
    assert_eq!(
        maxmind_web_evidence_source(),
        GeoEvidenceSource::RemoteLookup
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn caches_maxmind_web_lookup(pool: PgPool) {
    let body = json!({"country": {"iso_code": "FR"}});
    let (relation, location) = maxmind_ip_relation("203.0.113.7".parse().unwrap(), &body).unwrap();
    let lookup = MaxMindGeoLiteWebLookup { location, relation };

    let mut tx = pool.begin().await.expect("begin");
    cache_maxmind_geolite_web_tx(&mut tx, &lookup, Utc::now() + Duration::days(7))
        .await
        .expect("cache");
    tx.commit().await.expect("commit");

    let cached_country: Option<String> = sqlx::query_scalar(
        "SELECT country_code FROM geo_ip_network_relations WHERE source_code = 'maxmind_geolite_city_web'",
    )
    .fetch_one(&pool)
    .await
    .expect("country");
    assert_eq!(cached_country.as_deref(), Some("FR"));
}

#[tokio::test]
async fn web_client_lookup_uses_local_endpoint_fixture() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::MaxMindGeoLiteWebClient;
    use crate::geo::maxmind_types::MaxMindGeoLiteConfig;

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/geoip/v2.1/city/93.184.216.34"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "country": {"iso_code": "US"}
        })))
        .mount(&server)
        .await;

    let client = MaxMindGeoLiteWebClient::new(
        reqwest::Client::new(),
        MaxMindGeoLiteConfig {
            account_id: "account".to_string(),
            license_key: "license".to_string(),
            eula_accepted: true,
        },
    )
    .expect("config");

    let endpoint = format!("{}/geoip/v2.1/city", server.uri());
    let lookup = client
        .lookup_at("93.184.216.34".parse().unwrap(), &endpoint)
        .await
        .expect("lookup")
        .expect("payload");
    assert_eq!(lookup.location.country_code, "US");
}

#[tokio::test]
async fn web_client_lookup_returns_none_for_not_found() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::MaxMindGeoLiteWebClient;
    use crate::geo::maxmind_types::MaxMindGeoLiteConfig;

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/geoip/v2.1/city/93.184.216.34"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let client = MaxMindGeoLiteWebClient::new(
        reqwest::Client::new(),
        MaxMindGeoLiteConfig {
            account_id: "account".to_string(),
            license_key: "license".to_string(),
            eula_accepted: true,
        },
    )
    .expect("config");
    let endpoint = format!("{}/geoip/v2.1/city", server.uri());
    let lookup = client
        .lookup_at("93.184.216.34".parse().unwrap(), &endpoint)
        .await
        .expect("lookup");
    assert!(lookup.is_none());
}
