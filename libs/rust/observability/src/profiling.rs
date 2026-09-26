use nvbes_core::config::AppConfig;
use pyroscope::PyroscopeAgent;
use pyroscope::backend::{BackendConfig, PprofConfig, pprof_backend};
use pyroscope::pyroscope::{PyroscopeAgentBuilder, PyroscopeAgentReady, PyroscopeAgentRunning};

pub struct ContinuousProfilingGuard {
    agent: Option<PyroscopeAgent<PyroscopeAgentRunning>>,
}

impl Drop for ContinuousProfilingGuard {
    fn drop(&mut self) {
        let Some(agent) = self.agent.take() else {
            return;
        };

        finish_profiling_stop(agent.stop());
    }
}

fn finish_profiling_stop(
    result: Result<PyroscopeAgent<PyroscopeAgentReady>, impl std::fmt::Display>,
) {
    match result {
        Ok(agent) => agent.shutdown(),
        Err(error) => log_profiling_shutdown_failure(error),
    }
}

fn log_profiling_shutdown_failure(error: impl std::fmt::Display) {
    tracing::warn!(error = %error, "continuous profiling shutdown failed");
}

pub fn start_continuous_profiling(
    config: &AppConfig,
    service_name: &'static str,
) -> Result<Option<ContinuousProfilingGuard>, String> {
    if !config.profiling_enabled {
        return Ok(None);
    }

    let endpoint = config.profiling_endpoint.as_deref().ok_or_else(|| {
        "NVBES_PROFILING_ENDPOINT is required when profiling is enabled".to_string()
    })?;

    let sample_rate = config.profiling_sample_rate_hz;
    let pprof_config = PprofConfig { sample_rate };
    let mut builder = PyroscopeAgentBuilder::new(
        endpoint,
        service_name,
        sample_rate,
        "pyroscope-rs",
        env!("CARGO_PKG_VERSION"),
        pprof_backend(pprof_config, BackendConfig::default()),
    )
    .tags(vec![
        ("service", service_name),
        ("environment", config.environment.as_str()),
        ("platform", "nvbes"),
    ]);

    if let (Some(username), Some(password)) = (
        config.profiling_basic_auth_user.as_deref(),
        config.profiling_basic_auth_password.as_deref(),
    ) {
        builder = builder.basic_auth(username, password);
    }

    let agent = into_profiling_start_result(builder.build().and_then(|agent| agent.start()))?;

    tracing::info!(
        service = service_name,
        environment = %config.environment,
        sample_rate_hz = sample_rate,
        "continuous profiling started"
    );

    Ok(Some(ContinuousProfilingGuard { agent: Some(agent) }))
}

fn into_profiling_start_result(
    result: Result<PyroscopeAgent<PyroscopeAgentRunning>, impl std::fmt::Display>,
) -> Result<PyroscopeAgent<PyroscopeAgentRunning>, String> {
    result.map_err(map_profiling_start_error)
}

fn map_profiling_start_error(error: impl std::fmt::Display) -> String {
    format!("failed to start continuous profiling: {error}")
}

#[cfg(test)]
#[path = "observability.profiling.tests.rs"]
mod tests;
