#![cfg(feature = "profiling")]

use nvbes_core::config::AppConfig;
use nvbes_observability::start_continuous_profiling;

#[test]
fn starts_and_stops_continuous_profiling_with_basic_auth() {
    let config = AppConfig {
        environment: "development".to_string(),
        profiling_enabled: true,
        profiling_endpoint: Some("http://127.0.0.1:4040".to_string()),
        profiling_sample_rate_hz: 10,
        profiling_basic_auth_user: Some("stack".to_string()),
        profiling_basic_auth_password: Some("token".to_string()),
        ..AppConfig::default()
    };
    let guard = start_continuous_profiling(&config, "nvbes-observability-profiling-init")
        .expect("profiling agent should start against a local endpoint");
    assert!(guard.is_some());
    drop(guard);
}
