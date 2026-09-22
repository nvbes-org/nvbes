use nvbes_core::config::AppConfig;
use pyroscope::PyroscopeAgent;
use pyroscope::backend::{BackendConfig, PprofConfig, pprof_backend};
use pyroscope::pyroscope::{PyroscopeAgentBuilder, PyroscopeAgentRunning};

pub struct ContinuousProfilingGuard {
    agent: Option<PyroscopeAgent<PyroscopeAgentRunning>>,
}

impl Drop for ContinuousProfilingGuard {
    fn drop(&mut self) {
        let Some(agent) = self.agent.take() else {
            return;
        };

        match agent.stop() {
            Ok(agent) => agent.shutdown(),
            Err(error) => tracing::warn!(error = %error, "continuous profiling shutdown failed"),
        }
    }
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

    let agent = builder
        .build()
        .and_then(|agent| agent.start())
        .map_err(|error| format!("failed to start continuous profiling: {error}"))?;

    tracing::info!(
        service = service_name,
        environment = %config.environment,
        sample_rate_hz = sample_rate,
        "continuous profiling started"
    );

    Ok(Some(ContinuousProfilingGuard { agent: Some(agent) }))
}

#[cfg(test)]
#[path = "observability.profiling.tests.rs"]
mod tests;
