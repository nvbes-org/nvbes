use super::{
    validate_database_url, validate_jwt_secret, validate_profiling_endpoint, validate_public_url,
    validate_webauthn_rp_id,
};

#[test]
fn validate_database_url_strict_mode_requires_sslmode() {
    let err = validate_database_url("postgres://db.example.com/nvbes", true)
        .expect_err("missing sslmode");
    assert!(err.contains("sslmode"));

    validate_database_url("postgres://db.example.com/nvbes?sslmode=require", true)
        .expect("require accepted");
    validate_database_url("postgres://db.example.com/nvbes?sslmode=verify-full", true)
        .expect("verify-full accepted");

    let bad = validate_database_url("postgres://db.example.com/nvbes?sslmode=prefer", true)
        .expect_err("weak sslmode");
    assert!(bad.contains("prefer"));
}

#[test]
fn validate_database_url_allows_any_url_in_development() {
    validate_database_url("postgres://localhost/nvbes", false).expect("dev ok");
}

#[test]
fn validate_public_url_rejects_loopback_ip_in_strict_mode() {
    let err = validate_public_url("NVBES_API_BASE_URL", "https://127.0.0.1:4000", true, true)
        .expect_err("loopback ip");
    assert!(err.contains("loopback"));
}

#[test]
fn validate_public_url_rejects_http_when_https_required() {
    let err = validate_public_url("NVBES_WEB_BASE_URL", "http://app.example.com", true, true)
        .expect_err("http");
    assert!(err.contains("HTTPS"));
}

#[test]
fn validate_profiling_endpoint_rejects_unsupported_scheme() {
    let err = validate_profiling_endpoint("ftp://collector.example.com").expect_err("bad scheme");
    assert!(err.contains("HTTP or HTTPS"));
}

#[test]
fn validate_webauthn_rp_id_rejects_empty_value() {
    let err = validate_webauthn_rp_id("   ", false).expect_err("empty");
    assert!(err.contains("empty"));
}

#[test]
fn validate_jwt_secret_allows_any_value_in_development() {
    validate_jwt_secret("short", false).expect("dev mode");
}

#[test]
fn validate_public_url_rejects_localhost_subdomain_in_strict_mode() {
    let err = validate_public_url("NVBES_WEB_BASE_URL", "https://app.localhost", true, true)
        .expect_err("localhost subdomain");
    assert!(err.contains("localhost"));
}

#[test]
fn validate_webauthn_rp_id_rejects_loopback_ip_in_strict_mode() {
    let err = validate_webauthn_rp_id("127.0.0.1", true).expect_err("loopback");
    assert!(err.contains("loopback"));
}

#[test]
fn validate_database_url_rejects_invalid_url() {
    let err = validate_database_url("not-a-url", false).expect_err("invalid");
    assert!(err.contains("valid URL"));
}

#[test]
fn validate_public_url_covers_remaining_strict_branches() {
    validate_public_url("NVBES_WEB_BASE_URL", "http://app.example.com", true, false)
        .expect("http allowed when https not required");
    validate_public_url("NVBES_WEB_BASE_URL", "https://203.0.113.10", true, true)
        .expect("public ipv4 accepted");

    let ipv6 = validate_public_url("NVBES_API_BASE_URL", "https://[::1]", true, true)
        .expect_err("ipv6 loopback");
    assert!(ipv6.contains("loopback"));

    let invalid =
        validate_public_url("NVBES_WEB_BASE_URL", "not a url", true, true).expect_err("invalid");
    assert!(invalid.contains("valid URL"));
}

#[test]
fn validate_jwt_secret_and_profiling_and_webauthn_happy_paths() {
    validate_jwt_secret(&"a".repeat(32), true).expect("strict jwt ok");
    validate_profiling_endpoint("https://collector.example.com/ingest").expect("https");
    validate_profiling_endpoint("http://collector.example.com/ingest").expect("http");
    let no_host = validate_profiling_endpoint("https://").expect_err("empty host");
    assert!(no_host.contains("valid URL") || no_host.contains("host"));
    validate_webauthn_rp_id("localhost", false).expect("dev localhost");
    validate_webauthn_rp_id("login.example.com", true).expect("prod rp id");
    let ipv6 = validate_webauthn_rp_id("::1", true).expect_err("ipv6 loopback");
    assert!(ipv6.contains("loopback"));
}
