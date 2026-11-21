use super::{validate_client_id, validate_redirect_uri, validate_scope};

#[test]
fn validate_helpers_cover_boundary_lengths_and_punctuation() {
    assert!(validate_client_id(&"a".repeat(128)));
    assert!(!validate_client_id(&"a".repeat(129)));
    assert!(validate_client_id("edge-case_id"));

    let exact_https = format!("https://example.com/{}", "x".repeat(2048 - 20));
    assert_eq!(exact_https.len(), 2048);
    assert!(validate_redirect_uri(&exact_https));
    assert!(!validate_redirect_uri(&format!("{exact_https}y")));

    assert!(validate_scope("read-write my_scope account:read"));
    assert!(validate_scope(&"a".repeat(64)));
    assert!(!validate_scope(&"a".repeat(65)));
    assert!(validate_scope("   "));
}
