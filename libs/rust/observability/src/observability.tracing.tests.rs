use std::sync::{Mutex, OnceLock};

use super::{OtlpProtocol, TracingConfig, otlp_http_signal_endpoint, use_json_logs};

fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}

#[test]
fn http_signal_endpoint_is_canonical() {
    assert_eq!(
        otlp_http_signal_endpoint("https://example.grafana.net/otlp/", "/v1/traces"),
        "https://example.grafana.net/otlp/v1/traces"
    );
    assert_eq!(
        otlp_http_signal_endpoint("https://example.grafana.net/otlp", "v1/metrics"),
        "https://example.grafana.net/otlp/v1/metrics"
    );
}

#[test]
fn use_json_logs_honors_explicit_format_and_environment_default() {
    let _guard = env_lock();
    unsafe {
        std::env::remove_var("NVBES_LOG_FORMAT");
    }
    assert!(!use_json_logs("development"));
    assert!(use_json_logs("production"));
    assert!(use_json_logs("staging"));

    unsafe {
        std::env::set_var("NVBES_LOG_FORMAT", "json");
    }
    assert!(use_json_logs("development"));

    unsafe {
        std::env::set_var("NVBES_LOG_FORMAT", "compact");
    }
    assert!(!use_json_logs("production"));

    unsafe {
        std::env::set_var("NVBES_LOG_FORMAT", "other");
    }
    assert!(use_json_logs("production"));
    assert!(!use_json_logs("development"));

    unsafe {
        std::env::remove_var("NVBES_LOG_FORMAT");
    }
}

#[test]
fn tracing_config_defaults_to_grpc_protocol() {
    let config = TracingConfig {
        environment: "test",
        otlp_endpoint: None,
        otlp_authorization_header: None,
        protocol: OtlpProtocol::default(),
    };
    assert_eq!(config.protocol, OtlpProtocol::Grpc);
    assert_ne!(OtlpProtocol::Grpc, OtlpProtocol::Http);
}

#[cfg(feature = "otlp")]
#[test]
fn otlp_headers_and_metadata_honor_authorization_header() {
    use super::{otlp_headers, otlp_metadata};

    let empty = TracingConfig {
        environment: "test",
        otlp_endpoint: Some("https://otlp.example/"),
        otlp_authorization_header: None,
        protocol: OtlpProtocol::Http,
    };
    assert!(otlp_headers(empty).is_empty());
    assert!(otlp_metadata(empty).is_none());

    let blank = TracingConfig {
        otlp_authorization_header: Some("   "),
        ..empty
    };
    assert!(otlp_headers(blank).is_empty());
    assert!(otlp_metadata(blank).is_none());

    let auth = TracingConfig {
        otlp_authorization_header: Some("Basic dXNlcjpwYXNz"),
        ..empty
    };
    assert_eq!(
        otlp_headers(auth).get("authorization").map(String::as_str),
        Some("Basic dXNlcjpwYXNz")
    );
    let metadata = otlp_metadata(auth).expect("authorization metadata");
    assert!(metadata.get("authorization").is_some());
}
