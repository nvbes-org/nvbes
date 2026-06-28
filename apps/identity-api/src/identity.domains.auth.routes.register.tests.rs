use axum::http::{HeaderMap, HeaderValue, header};

use super::{
    RegisterRequest, if_none_match_matches, register_input_from_request, supported_region_catalog,
    supported_regions_cache,
};

#[test]
fn register_input_from_request_maps_http_payload_to_onboarding_input() {
    let birthdate = chrono::NaiveDate::from_ymd_opt(1990, 5, 4).expect("valid date");
    let input = register_input_from_request(
        RegisterRequest {
            email: " User@Example.COM ".to_string(),
            password: "Secret123!".to_string(),
            firstname: "Ada".to_string(),
            lastname: "Lovelace".to_string(),
            username: "ada".to_string(),
            birthdate: Some("1990-05-04".to_string()),
            region: Some("FR".to_string()),
            workspace_name: "Ada Workspace".to_string(),
            pow_nonce: "nonce".to_string(),
            pow_solution: "solution".to_string(),
            legal_documents_accepted: true,
            marketing_emails_accepted: true,
        },
        "FR".to_string(),
        "eu".to_string(),
        Some(birthdate),
        Some("203.0.113.10".to_string()),
        Some("nvbes-test".to_string()),
    );

    assert_eq!(input.email, " User@Example.COM ");
    assert_eq!(input.password, "Secret123!");
    assert_eq!(input.firstname, "Ada");
    assert_eq!(input.lastname, "Lovelace");
    assert_eq!(input.username, "ada");
    assert_eq!(input.birthdate, Some(birthdate));
    assert_eq!(input.region.as_deref(), Some("FR"));
    assert_eq!(input.data_region.as_deref(), Some("eu"));
    assert_eq!(input.workspace_name, "Ada Workspace");
    assert_eq!(input.ip.as_deref(), Some("203.0.113.10"));
    assert_eq!(input.user_agent.as_deref(), Some("nvbes-test"));
    assert!(input.legal_documents_accepted);
    assert!(input.marketing_emails_accepted);
}

#[test]
fn supported_regions_cache_uses_strong_etag() {
    let cache = supported_regions_cache();

    assert!(cache.etag.starts_with("\"regions-"));
    assert!(cache.etag.ends_with('"'));
    assert!(!cache.body.is_empty());
}

#[test]
fn supported_regions_catalog_is_sorted_for_stable_hashing() {
    let catalog = supported_region_catalog();
    let mut sorted_codes: Vec<_> = catalog.iter().map(|region| region.country_code).collect();
    sorted_codes.sort_unstable();

    assert_eq!(
        catalog
            .iter()
            .map(|region| region.country_code)
            .collect::<Vec<_>>(),
        sorted_codes
    );
}

#[test]
fn if_none_match_accepts_matching_etag_and_wildcard() {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::IF_NONE_MATCH,
        HeaderValue::from_static("\"other\", \"regions-test\""),
    );

    assert!(if_none_match_matches(&headers, "\"regions-test\""));

    headers.insert(header::IF_NONE_MATCH, HeaderValue::from_static("*"));

    assert!(if_none_match_matches(&headers, "\"regions-test\""));
}
