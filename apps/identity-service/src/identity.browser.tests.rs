use axum::{
    Extension, Router,
    body::Body,
    http::{HeaderMap, HeaderValue, Method, Request, StatusCode},
    routing::post,
};
use tower::ServiceExt;

use super::{BrowserProof, BrowserSecurity, protect_mutation};
use crate::oauth::store::random_secret;

fn headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    for (name, value) in [
        ("origin", "https://identity.example"),
        ("content-type", "application/json"),
        ("sec-fetch-site", "same-origin"),
        ("sec-fetch-mode", "cors"),
        ("sec-fetch-dest", "empty"),
    ] {
        headers.insert(name, HeaderValue::from_static(value));
    }
    headers.insert(
        "x-csrf-token",
        HeaderValue::from_str(&random_secret()).unwrap(),
    );
    headers.insert(
        "cookie",
        HeaderValue::from_str(&format!(
            "__Host-nvbes-browser={}; __Host-nvbes-session={}",
            random_secret(),
            random_secret()
        ))
        .unwrap(),
    );
    headers
}

#[test]
fn origins_and_development_cookie_names_cannot_downgrade_production() {
    for origin in [
        "http://identity.example",
        "https://identity.example/other",
        "https://user@identity.example",
        "https://identity.example?x=1",
        "https://identity.example/#x",
        "https://identity.example\\evil",
    ] {
        assert!(BrowserSecurity::new(origin, false).is_err(), "{origin}");
    }
    assert!(BrowserSecurity::new("http://localhost:3000", false).is_err());
    assert!(BrowserSecurity::new("http://identity.example", true).is_err());
    let production = BrowserSecurity::new("https://identity.example", false).unwrap();
    let cookie = production
        .browser_cookie()
        .header
        .to_str()
        .unwrap()
        .to_owned();
    assert!(cookie.starts_with("__Host-nvbes-browser="));
    for attribute in [
        "Path=/",
        "HttpOnly",
        "SameSite=Lax",
        "Secure",
        "Max-Age=3600",
    ] {
        assert!(cookie.contains(attribute));
    }
    assert!(!cookie.contains("Domain="));
    assert!(
        production
            .clear_session_cookie()
            .to_str()
            .unwrap()
            .contains("Max-Age=0")
    );
    assert!(production.session_cookie("malformed; Secure").is_err());
    let development = BrowserSecurity::new("http://localhost:3000", true).unwrap();
    assert!(
        development
            .browser_cookie()
            .header
            .to_str()
            .unwrap()
            .starts_with("nvbes-dev-browser=")
    );
    let mut headers = headers();
    headers.insert(
        "cookie",
        HeaderValue::from_str(&format!("nvbes-dev-browser={}", random_secret())).unwrap(),
    );
    assert!(production.verify_mutation(&Method::POST, &headers).is_err());
}

#[test]
fn only_explicit_same_origin_json_mutations_produce_evidence() {
    let security = BrowserSecurity::new("https://identity.example", false).unwrap();
    assert!(security.verify_mutation(&Method::POST, &headers()).is_ok());
    for method in [Method::GET, Method::PUT, Method::DELETE] {
        assert!(security.verify_mutation(&method, &headers()).is_err());
    }
    for (name, value) in [
        ("origin", "https://account.example"),
        ("origin", "https://identity.example.evil"),
        ("origin", "null"),
        ("origin", "http://identity.example"),
        ("sec-fetch-site", "same-site"),
        ("sec-fetch-site", "cross-site"),
        ("sec-fetch-mode", "navigate"),
        ("sec-fetch-dest", "iframe"),
        ("content-type", "text/plain"),
        ("content-type", "application/x-www-form-urlencoded"),
        ("x-csrf-token", "short"),
    ] {
        let mut changed = headers();
        changed.insert(name, HeaderValue::from_static(value));
        assert!(
            security.verify_mutation(&Method::POST, &changed).is_err(),
            "{name}: {value}"
        );
    }
    for name in ["origin", "cookie", "content-type", "x-csrf-token"] {
        let mut changed = headers();
        changed.remove(name);
        assert!(security.verify_mutation(&Method::POST, &changed).is_err());
    }
    // Older browsers may omit Fetch Metadata; exact Origin and CSRF stay mandatory.
    let mut changed = headers();
    for name in ["sec-fetch-site", "sec-fetch-mode", "sec-fetch-dest"] {
        changed.remove(name);
    }
    assert!(security.verify_mutation(&Method::POST, &changed).is_ok());
}

#[test]
fn duplicate_headers_and_cookie_shadowing_are_rejected() {
    let security = BrowserSecurity::new("https://identity.example", false).unwrap();
    for name in [
        "origin",
        "content-type",
        "x-csrf-token",
        "sec-fetch-site",
        "cookie",
    ] {
        let mut changed = headers();
        changed.append(name, changed.get(name).unwrap().clone());
        assert!(
            security.verify_mutation(&Method::POST, &changed).is_err(),
            "{name}"
        );
    }
    let mut changed = headers();
    changed.append(
        "cookie",
        HeaderValue::from_str(&format!(
            "unrelated=ok; __Host-nvbes-session={}",
            random_secret()
        ))
        .unwrap(),
    );
    assert!(security.verify_mutation(&Method::POST, &changed).is_err());
    changed.insert(
        "cookie",
        HeaderValue::from_str(&format!(
            "__Host-nvbes-browser={}; __Host-nvbes-browser={}",
            random_secret(),
            random_secret()
        ))
        .unwrap(),
    );
    assert!(security.verify_mutation(&Method::POST, &changed).is_err());
}

#[tokio::test]
async fn axum_boundary_rejects_before_handler_and_applies_response_protections() {
    let security = BrowserSecurity::new("https://identity.example", false).unwrap();
    let app = Router::new()
        .route(
            "/consent",
            post(|Extension(_): Extension<BrowserProof>| async { StatusCode::NO_CONTENT }),
        )
        .layer(axum::middleware::from_fn_with_state(
            security,
            protect_mutation,
        ));
    for valid in [true, false] {
        let mut request = Request::builder()
            .uri("/consent")
            .method(Method::POST)
            .body(Body::empty())
            .unwrap();
        *request.headers_mut() = headers();
        if !valid {
            request.headers_mut().insert(
                "origin",
                HeaderValue::from_static("https://account.example"),
            );
        }
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(
            response.status(),
            if valid {
                StatusCode::NO_CONTENT
            } else {
                StatusCode::FORBIDDEN
            }
        );
        assert_eq!(response.headers()["cache-control"], "no-store");
        assert_eq!(response.headers()["referrer-policy"], "no-referrer");
        assert_eq!(response.headers()["x-frame-options"], "DENY");
        assert!(
            response.headers()["content-security-policy"]
                .to_str()
                .unwrap()
                .contains("frame-ancestors 'none'")
        );
        assert!(
            !response
                .headers()
                .contains_key("access-control-allow-origin")
        );
    }
}
