use nvbes_core::config::AppConfig;
use nvbes_observability::init_tracing;

#[test]
fn init_tracing_uses_app_config_defaults() {
    let config = AppConfig {
        environment: "development".to_string(),
        app_name: "nvbes-observability-app-config".to_string(),
        otlp_endpoint: None,
        ..AppConfig::default()
    };
    init_tracing(&config);
}
