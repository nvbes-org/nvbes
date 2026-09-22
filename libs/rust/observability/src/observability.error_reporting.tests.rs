use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use nvbes_core::config::AppConfig;

use super::{
    ErrorReportingConfig, HttpServerErrorContext, capture_http_server_error, error_reporting_test_lock,
    flush_error_reporting, init_error_reporting, init_error_reporting_for_service,
    init_error_reporting_with_config, install_safe_panic_hook, is_error_reporting_configured,
    release_name,
};

fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}

#[test]
fn init_without_dsn_disables_error_reporting() {
    let _lock = error_reporting_test_lock();
    let _guard = init_error_reporting_with_config(ErrorReportingConfig {
        app_name: "nvbes-observability-tests",
        service_name: "nvbes-observability-tests",
        environment: "test",
        dsn: None,
        traces_sample_rate: 0.0,
    });
    assert!(!is_error_reporting_configured());
    capture_http_server_error(&HttpServerErrorContext {
        method: "GET",
        path_template: "/health",
        status: 500,
        request_id: "req_test",
        trace_id: "4bf92f3577b34da6a3ce929d0e0e4736",
        span_id: "00f067aa0ba902b7",
        duration_ms: 12,
    });
    assert!(!flush_error_reporting(Duration::from_millis(10)));
    install_safe_panic_hook();
}

#[test]
fn error_reporting_config_fields_are_accessible() {
    let config = ErrorReportingConfig {
        app_name: "app",
        service_name: "svc",
        environment: "staging",
        dsn: Some("https://public@o.ingest.sentry.io/1"),
        traces_sample_rate: 0.25,
    };
    assert_eq!(config.app_name, "app");
    assert_eq!(config.service_name, "svc");
    assert_eq!(config.environment, "staging");
    assert_eq!(config.traces_sample_rate, 0.25);
    assert!(config.dsn.is_some());
}

#[test]
fn init_from_app_config_without_dsn_stays_disabled() {
    let _lock = error_reporting_test_lock();
    let config = AppConfig {
        app_name: "nvbes-observability".into(),
        environment: "test".into(),
        sentry_dsn: None,
        sentry_traces_sample_rate: 0.0,
        ..AppConfig::default()
    };
    let _guard = init_error_reporting(&config);
    assert!(!is_error_reporting_configured());
    let _guard = init_error_reporting_for_service(&config, "custom-service");
    assert!(!is_error_reporting_configured());
}

#[test]
fn release_name_prefers_sentry_then_nvbes_env() {
    let _guard = env_lock();
    unsafe {
        std::env::remove_var("SENTRY_RELEASE");
        std::env::remove_var("NVBES_RELEASE");
        std::env::set_var("SENTRY_RELEASE", "sentry-rel-1");
    }
    assert_eq!(release_name().as_deref(), Some("sentry-rel-1"));

    unsafe {
        std::env::remove_var("SENTRY_RELEASE");
        std::env::set_var("NVBES_RELEASE", "nvbes-rel-2");
    }
    assert_eq!(release_name().as_deref(), Some("nvbes-rel-2"));

    unsafe {
        std::env::remove_var("SENTRY_RELEASE");
        std::env::remove_var("NVBES_RELEASE");
    }
    let fallback = release_name();
    assert!(fallback.is_some());
}

#[test]
fn init_with_dsn_enables_capture_path() {
    let _lock = error_reporting_test_lock();
    let _guard = init_error_reporting_with_config(ErrorReportingConfig {
        app_name: "nvbes-observability-tests",
        service_name: "nvbes-observability-tests",
        environment: "test",
        dsn: Some("https://public@127.0.0.1/1"),
        traces_sample_rate: 0.0,
    });
    assert!(is_error_reporting_configured());
    capture_http_server_error(&HttpServerErrorContext {
        method: "POST",
        path_template: "/api/v1/items",
        status: 503,
        request_id: "req_dsn",
        trace_id: "4bf92f3577b34da6a3ce929d0e0e4736",
        span_id: "00f067aa0ba902b7",
        duration_ms: 40,
    });
    let _ = flush_error_reporting(Duration::from_millis(50));
}
