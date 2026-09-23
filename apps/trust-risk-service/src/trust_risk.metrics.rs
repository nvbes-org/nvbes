use std::{
    collections::HashMap,
    sync::atomic::{AtomicBool, Ordering},
    sync::{Arc, OnceLock},
    time::Duration,
};

use axum::{
    Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::get,
};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use opentelemetry::{KeyValue, global};
use opentelemetry_otlp::{WithExportConfig, WithHttpConfig};
use opentelemetry_sdk::{Resource, metrics::SdkMeterProvider};

use crate::{
    app::TrustRiskState, auth::constant_time_eq, config::TrustRiskConfig, error_reporting::APP_NAME,
};

static OTLP_METRICS_CONFIGURED: AtomicBool = AtomicBool::new(false);

pub struct GrafanaMetricsGuard {
    provider: Option<SdkMeterProvider>,
}

impl Drop for GrafanaMetricsGuard {
    fn drop(&mut self) {
        if let Some(provider) = self.provider.take()
            && let Err(error) = provider.shutdown()
        {
            tracing::warn!(%error, "Grafana OTLP metrics shutdown failed");
        }
        OTLP_METRICS_CONFIGURED.store(false, Ordering::Relaxed);
    }
}

struct OtelRiskMetrics {
    signals: opentelemetry::metrics::Counter<u64>,
    assessments: opentelemetry::metrics::Counter<u64>,
    assessment_duration: opentelemetry::metrics::Histogram<f64>,
    projection_events: opentelemetry::metrics::Counter<u64>,
}

pub fn init_otlp(config: &TrustRiskConfig) -> anyhow::Result<GrafanaMetricsGuard> {
    let Some(endpoint) = config.otlp_endpoint.as_deref() else {
        return Ok(GrafanaMetricsGuard { provider: None });
    };

    let headers = config
        .otlp_authorization_header
        .as_deref()
        .map(|authorization| {
            HashMap::from([("authorization".to_string(), authorization.to_string())])
        })
        .unwrap_or_default();
    let exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_http()
        .with_endpoint(nvbes_observability::otlp_http_signal_endpoint(
            endpoint,
            "v1/metrics",
        ))
        .with_headers(headers);
    let exporter = exporter.build()?;
    let provider = SdkMeterProvider::builder()
        .with_periodic_exporter(exporter)
        .with_resource(
            Resource::builder()
                .with_service_name(APP_NAME)
                .with_attributes([
                    KeyValue::new("service.namespace", "nvbes"),
                    KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
                    KeyValue::new("deployment.environment", config.environment.clone()),
                    KeyValue::new("deployment.environment.name", config.environment.clone()),
                ])
                .build(),
        )
        .build();
    global::set_meter_provider(provider.clone());
    OTLP_METRICS_CONFIGURED.store(true, Ordering::Relaxed);
    tracing::info!("Grafana OTLP metrics enabled");
    Ok(GrafanaMetricsGuard {
        provider: Some(provider),
    })
}

fn otel() -> Option<&'static OtelRiskMetrics> {
    static METRICS: OnceLock<OtelRiskMetrics> = OnceLock::new();
    OTLP_METRICS_CONFIGURED.load(Ordering::Relaxed).then(|| {
        METRICS.get_or_init(|| {
            let meter = global::meter(APP_NAME);
            OtelRiskMetrics {
                signals: meter.u64_counter("trust_risk_signals_total").build(),
                assessments: meter.u64_counter("trust_risk_assessments_total").build(),
                assessment_duration: meter
                    .f64_histogram("trust_risk_assessment_duration_seconds")
                    .with_unit("s")
                    .build(),
                projection_events: meter
                    .u64_counter("trust_risk_projection_events_total")
                    .build(),
            }
        })
    })
}

pub fn install() -> Arc<PrometheusHandle> {
    static HANDLE: OnceLock<Arc<PrometheusHandle>> = OnceLock::new();
    HANDLE
        .get_or_init(|| {
            Arc::new(
                PrometheusBuilder::new()
                    .install_recorder()
                    .expect("trust/risk Prometheus recorder must install"),
            )
        })
        .clone()
}

pub fn router(state: TrustRiskState) -> Router {
    Router::new()
        .route("/metrics", get(render))
        .with_state(state)
}

async fn render(State(state): State<TrustRiskState>, headers: HeaderMap) -> impl IntoResponse {
    let provided = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .unwrap_or_default();
    if !constant_time_eq(provided.as_bytes(), state.config.metrics_token.as_bytes()) {
        return (StatusCode::UNAUTHORIZED, String::new());
    }
    (StatusCode::OK, state.metrics.render())
}

pub fn signal(producer: &str, family: &str, outcome: &'static str) {
    metrics::counter!("trust_risk_signals_total", "producer" => producer.to_string(), "family" => family.to_string(), "outcome" => outcome).increment(1);
    if let Some(otel) = otel() {
        otel.signals.add(
            1,
            &[
                KeyValue::new("producer", producer.to_string()),
                KeyValue::new("family", family.to_string()),
                KeyValue::new("outcome", outcome),
            ],
        );
    }
}

pub fn assessment(recommendation: &str, outcome: &'static str, duration: Duration) {
    let labels = [
        ("recommendation", recommendation.to_string()),
        ("outcome", outcome.to_string()),
    ];
    metrics::counter!("trust_risk_assessments_total", &labels).increment(1);
    metrics::histogram!("trust_risk_assessment_duration_seconds", &labels)
        .record(duration.as_secs_f64());
    if let Some(otel) = otel() {
        let attributes = [
            KeyValue::new("recommendation", recommendation.to_string()),
            KeyValue::new("outcome", outcome),
        ];
        otel.assessments.add(1, &attributes);
        otel.assessment_duration
            .record(duration.as_secs_f64(), &attributes);
    }
}

pub fn projection(outcome: &'static str, count: u64) {
    metrics::counter!("trust_risk_projection_events_total", "outcome" => outcome).increment(count);
    if let Some(otel) = otel() {
        otel.projection_events
            .add(count, &[KeyValue::new("outcome", outcome)]);
    }
}

#[cfg(test)]
#[path = "trust_risk.metrics.tests.rs"]
mod tests;
