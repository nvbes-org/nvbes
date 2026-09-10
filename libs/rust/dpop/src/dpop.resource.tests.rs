use super::*;
use crate::{generate_key_pair, proof::create_dpop_proof};

#[test]
fn resource_binding_cannot_be_downgraded_or_redirected_by_headers() {
    let key = generate_key_pair();
    let verifier = ResourceVerifier::new("https://api.example").unwrap();
    let proof = create_dpop_proof(
        &key,
        "GET",
        "https://api.example/profile",
        Some("token"),
        None,
    )
    .unwrap();
    let mut headers = HeaderMap::new();
    headers.insert("authorization", "DPoP token".parse().unwrap());
    headers.insert("dpop", proof.parse().unwrap());
    headers.insert("host", "evil.example".parse().unwrap());
    headers.insert("x-forwarded-host", "evil.example".parse().unwrap());
    let credentials = Credentials::from_headers(&headers).unwrap();
    let verify = |jkt, method, path: &str| {
        ResourceVerifier::verify(
            Some(&verifier),
            &credentials,
            jkt,
            &method,
            &path.parse().unwrap(),
        )
    };
    assert!(verify(Some(key.jkt.as_str()), Method::GET, "/profile?sort=asc").is_ok());
    assert!(verify(Some(key.jkt.as_str()), Method::POST, "/profile").is_err());
    assert!(verify(Some(key.jkt.as_str()), Method::GET, "/other").is_err());
    assert!(
        verify(
            Some(key.jkt.as_str()),
            Method::GET,
            "https://evil.example/profile"
        )
        .is_err()
    );
    assert!(verify(None, Method::GET, "/profile").is_err());
    let other = generate_key_pair();
    assert!(verify(Some(other.jkt.as_str()), Method::GET, "/profile").is_err());
    assert!(matches!(
        ResourceVerifier::verify(
            None,
            &credentials,
            Some(&key.jkt),
            &Method::GET,
            &"/profile".parse().unwrap()
        ),
        Err(ResourceError::Unavailable)
    ));
    headers.insert("authorization", "Bearer token".parse().unwrap());
    assert!(Credentials::from_headers(&headers).is_err());
    headers.remove("dpop");
    let bearer = Credentials::from_headers(&headers).unwrap();
    assert!(
        ResourceVerifier::verify(
            Some(&verifier),
            &bearer,
            Some(&key.jkt),
            &Method::GET,
            &"/profile".parse().unwrap()
        )
        .is_err()
    );
    assert!(
        ResourceVerifier::verify(
            None,
            &bearer,
            None,
            &Method::GET,
            &"/profile".parse().unwrap()
        )
        .unwrap()
        .is_none()
    );
    headers.append("authorization", "Bearer second".parse().unwrap());
    assert!(Credentials::from_headers(&headers).is_err());
}

#[test]
fn bindings_and_origins_are_strict() {
    for origin in [
        "http://api.example",
        "https://user:pass@api.example",
        "https://api.example/base",
        "https://api.example/?x",
        "https://api.example/#x",
    ] {
        assert!(ResourceVerifier::new(origin).is_err());
    }
    for value in [
        serde_json::json!({}),
        serde_json::json!({"jkt":"short"}),
        serde_json::json!({"jkt":"A".repeat(43),"extra":true}),
    ] {
        assert!(binding(Some(&value)).is_err());
    }
}
