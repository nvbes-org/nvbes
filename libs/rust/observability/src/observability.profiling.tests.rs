use nvbes_core::config::AppConfig;
use pyroscope::PyroscopeAgent;
use pyroscope::pyroscope::{PyroscopeAgentReady, PyroscopeAgentRunning};

use super::{ContinuousProfilingGuard, start_continuous_profiling};

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

#[test]
fn continuous_profiling_guard_drop_is_noop_without_agent() {
    let guard = ContinuousProfilingGuard { agent: None };
    drop(guard);
}

#[test]
fn profiling_starts_without_basic_auth_when_endpoint_is_set() {
    let config = AppConfig {
        environment: "development".to_string(),
        profiling_enabled: true,
        profiling_endpoint: Some("http://127.0.0.1:4040".to_string()),
        profiling_sample_rate_hz: 10,
        profiling_basic_auth_user: None,
        profiling_basic_auth_password: Some("ignored-without-user".to_string()),
        ..AppConfig::default()
    };
    let guard = start_continuous_profiling(&config, "nvbes-observability-profiling-no-auth")
        .expect("profiling agent should start without basic auth");
    assert!(guard.is_some());
    drop(guard);
}

#[test]
fn profiling_error_helpers_cover_failure_branches() {
    let start_err =
        match super::into_profiling_start_result(
            Result::<PyroscopeAgent<PyroscopeAgentRunning>, _>::Err("backend unavailable"),
        ) {
            Ok(_) => panic!("start errors must map"),
            Err(err) => err,
        };
    assert!(start_err.contains("failed to start continuous profiling"));
    assert!(start_err.contains("backend unavailable"));

    super::finish_profiling_stop(Result::<PyroscopeAgent<PyroscopeAgentReady>, _>::Err(
        "stop channel closed",
    ));
}
