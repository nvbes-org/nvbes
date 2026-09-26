#![cfg(feature = "otlp")]

use nvbes_core::config::AppConfig;
use nvbes_observability::init_tracing_for_service;

#[tokio::test]
async fn init_tracing_for_service_accepts_an_explicit_service_name() {
    let config = AppConfig {
        environment: "development".to_string(),
        app_name: "ignored".to_string(),
        otlp_endpoint: Some("http://127.0.0.1:4317".to_string()),
        ..AppConfig::default()
    };
    init_tracing_for_service(&config, "nvbes-observability-explicit-service");
}
