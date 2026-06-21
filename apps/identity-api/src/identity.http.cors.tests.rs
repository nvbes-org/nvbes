use super::{origin::expand_loopback_aliases, origin_from_redirect_uri, same_origin};

#[test]
fn origin_from_redirect_uri_extracts_scheme_host_and_port() {
    assert_eq!(
        origin_from_redirect_uri("http://localhost:5173/callback?code=abc"),
        Some("http://localhost:5173".to_string())
    );

    assert_eq!(
        origin_from_redirect_uri("http://[::1]:5173/callback?code=abc"),
        Some("http://[::1]:5173".to_string())
    );
}

#[test]
fn same_origin_accepts_loopback_aliases() {
    assert!(same_origin(
        "http://127.0.0.1:3001/path",
        "http://localhost:3001"
    ));
}

#[test]
fn expand_loopback_aliases_keeps_original_and_aliases() {
    let origins = expand_loopback_aliases(vec![
        "https://files.example.com".to_string(),
        "http://localhost:3001".to_string(),
    ]);

    assert!(
        origins
            .iter()
            .any(|origin| origin == "https://files.example.com")
    );
    assert!(
        origins
            .iter()
            .any(|origin| origin == "http://127.0.0.1:3001")
    );
}
