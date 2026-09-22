use nvbes_core::config::AppConfig;

use super::start_continuous_profiling;

#[test]
fn profiling_disabled_returns_none_without_endpoint() {
    let config = AppConfig {
        profiling_enabled: false,
        profiling_endpoint: None,
        ..AppConfig::default()
    };
    let guard = start_continuous_profiling(&config, "nvbes-observability-tests")
        .expect("disabled profiling is a no-op");
    assert!(guard.is_none());
}

#[test]
fn profiling_enabled_requires_endpoint() {
    let config = AppConfig {
        profiling_enabled: true,
        profiling_endpoint: None,
        ..AppConfig::default()
    };
    match start_continuous_profiling(&config, "nvbes-observability-tests") {
        Ok(_) => panic!("missing endpoint must fail"),
        Err(err) => assert!(err.contains("NVBES_PROFILING_ENDPOINT")),
    }
}
