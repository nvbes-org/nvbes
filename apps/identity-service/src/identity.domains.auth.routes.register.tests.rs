use axum::http::{HeaderMap, HeaderValue};

use super::{RegisterRequest, register_input_from_request, registration_country_from_headers};

#[test]
fn register_input_from_request_maps_http_payload_to_onboarding_input() {
    let input = register_input_from_request(
        RegisterRequest {
            email: " User@Example.COM ".to_string(),
            password: "Secret123!".to_string(),
            pow_nonce: "nonce".to_string(),
            pow_solution: "solution".to_string(),
            legal_documents_accepted: true,
            marketing_emails_accepted: true,
        },
        "eu".to_string(),
        Some("203.0.113.10".to_string()),
        Some("nvbes-test".to_string()),
    );

    assert_eq!(input.email, " User@Example.COM ");
    assert_eq!(input.password, "Secret123!");
    assert_eq!(input.data_region.as_deref(), Some("eu"));
    assert_eq!(input.ip.as_deref(), Some("203.0.113.10"));
    assert_eq!(input.user_agent.as_deref(), Some("nvbes-test"));
    assert!(input.legal_documents_accepted);
    assert!(input.marketing_emails_accepted);
}

#[test]
fn registration_region_is_resolved_by_the_backend() {
    let mut headers = HeaderMap::new();
    headers.insert("CF-IPCountry", HeaderValue::from_static("US"));

    assert_eq!(
        registration_country_from_headers(&headers, "production").as_deref(),
        Some("US")
    );
}

#[test]
fn local_registration_uses_a_deterministic_backend_region() {
    assert_eq!(
        registration_country_from_headers(&HeaderMap::new(), "development").as_deref(),
        Some("FR")
    );
    assert_eq!(
        registration_country_from_headers(&HeaderMap::new(), "production"),
        None
    );
}
