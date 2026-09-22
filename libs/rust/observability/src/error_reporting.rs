use std::{
    borrow::Cow,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use nvbes_core::config::AppConfig;
use sentry::protocol::{Level, Value};

static ERROR_REPORTING_CONFIGURED: AtomicBool = AtomicBool::new(false);

pub struct ErrorReportingGuard {
    _guard: Option<sentry::ClientInitGuard>,
}

#[derive(Debug, Clone, Copy)]
pub struct ErrorReportingConfig<'a> {
    pub app_name: &'a str,
    pub service_name: &'a str,
    pub environment: &'a str,
    pub dsn: Option<&'a str>,
    pub traces_sample_rate: f32,
}

pub struct HttpServerErrorContext<'a> {
    pub method: &'a str,
    pub path_template: &'a str,
    pub status: u16,
    pub request_id: &'a str,
    pub trace_id: &'a str,
    pub span_id: &'a str,
    pub duration_ms: u64,
}

pub fn init_error_reporting(config: &AppConfig) -> ErrorReportingGuard {
    init_error_reporting_for_service(config, &config.app_name)
}

pub fn init_error_reporting_for_service(
    config: &AppConfig,
    service_name: &str,
) -> ErrorReportingGuard {
    init_error_reporting_with_config(ErrorReportingConfig {
        app_name: &config.app_name,
        service_name,
        environment: &config.environment,
        dsn: config.sentry_dsn.as_deref(),
        traces_sample_rate: config.sentry_traces_sample_rate,
    })
}

pub fn init_error_reporting_with_config(config: ErrorReportingConfig<'_>) -> ErrorReportingGuard {
    let Some(dsn) = config.dsn else {
        ERROR_REPORTING_CONFIGURED.store(false, Ordering::Relaxed);
        tracing::info!("Sentry error reporting disabled: SENTRY_DSN is not set");
        return ErrorReportingGuard { _guard: None };
    };

    let guard = sentry::init((
        dsn,
        sentry::ClientOptions::default()
            .maybe_release(release_name())
            .environment(config.environment.to_owned())
            .traces_sample_rate(config.traces_sample_rate)
            .send_default_pii(false),
    ));

    sentry::configure_scope(|scope| {
        scope.set_tag("app.name", config.app_name);
        scope.set_tag("service.name", config.service_name);
        scope.set_tag("runtime", "rust");
    });

    ERROR_REPORTING_CONFIGURED.store(true, Ordering::Relaxed);
    tracing::info!(
        service_name = config.service_name,
        "Sentry error reporting enabled"
    );
    ErrorReportingGuard {
        _guard: Some(guard),
    }
}

pub fn is_error_reporting_configured() -> bool {
    ERROR_REPORTING_CONFIGURED.load(Ordering::Relaxed)
}

pub fn capture_http_server_error(context: &HttpServerErrorContext<'_>) {
    if !is_error_reporting_configured() {
        return;
    }

    sentry::with_scope(
        |scope| {
            scope.set_tag("http.method", context.method);
            scope.set_tag("http.route", context.path_template);
            scope.set_tag("http.status_code", context.status);
            scope.set_tag("request_id", context.request_id);
            scope.set_tag("trace_id", context.trace_id);
            scope.set_tag("span_id", context.span_id);
            scope.set_extra("duration_ms", Value::from(context.duration_ms));
        },
        || sentry::capture_message("HTTP request returned 5xx", Level::Error),
    );
}

pub fn flush_error_reporting(timeout: Duration) -> bool {
    sentry::Hub::with_active(|hub| {
        hub.client()
            .map(|client| client.flush(Some(timeout)))
            .unwrap_or(false)
    })
}

pub fn install_safe_panic_hook() {}

pub(crate) fn release_name() -> Option<Cow<'static, str>> {
    std::env::var("SENTRY_RELEASE")
        .or_else(|_| std::env::var("NVBES_RELEASE"))
        .ok()
        .map(Cow::Owned)
        .or_else(|| sentry::release_name!())
}

#[cfg(test)]
#[path = "observability.error_reporting.tests.rs"]
mod tests;
