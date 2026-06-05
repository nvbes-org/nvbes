use axum::{Router, extract::State, middleware, response::IntoResponse, routing::get};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use nvbes_core::config::AppConfig;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::{net::TcpListener, task::JoinHandle};

#[derive(Clone)]
pub struct HttpMetrics {
    pub handle: Arc<PrometheusHandle>,
}

impl HttpMetrics {
    pub fn new() -> Self {
        static PROMETHEUS_HANDLE: OnceLock<Arc<PrometheusHandle>> = OnceLock::new();
        let handle = PROMETHEUS_HANDLE
            .get_or_init(|| {
                Arc::new(
                    PrometheusBuilder::new()
                        .install_recorder()
                        .expect("failed to install Prometheus recorder"),
                )
            })
            .clone();
        Self { handle }
    }

    pub fn start_request(&self) {
        metrics::gauge!("http_active_requests").increment(1.0);
    }

    pub fn finish_request(
        &self,
        method: &str,
        path_template: &str,
        status: u16,
        duration: Duration,
    ) {
        metrics::gauge!("http_active_requests").decrement(1.0);

        let labels = [
            ("method", method.to_string()),
            ("path", path_template.to_string()),
            ("status", status.to_string()),
        ];

        metrics::counter!("http_requests_total", &labels).increment(1);
        metrics::histogram!("http_request_duration_seconds", &labels)
            .record(duration.as_secs_f64());
    }

    pub fn record_postgres_pool(&self, service: &str, environment: &str, size: u32, idle: usize) {
        let labels = [
            ("service", service.to_string()),
            ("environment", environment.to_string()),
        ];

        metrics::gauge!("postgres_pool_size", &labels).set(size as f64);
        metrics::gauge!("postgres_pool_idle", &labels).set(idle as f64);
    }

    pub fn record_upload_operation(
        &self,
        operation: &str,
        outcome: &str,
        size_bytes: Option<u64>,
        duration: Duration,
    ) {
        let labels = [
            ("operation", operation.to_string()),
            ("outcome", outcome.to_string()),
        ];

        metrics::counter!("upload_operations_total", &labels).increment(1);
        metrics::histogram!("upload_operation_duration_seconds", &labels)
            .record(duration.as_secs_f64());
        if let Some(size_bytes) = size_bytes {
            metrics::histogram!("upload_operation_size_bytes", &labels).record(size_bytes as f64);
        }
    }

    pub fn record_download_operation(
        &self,
        operation: &str,
        outcome: &str,
        size_bytes: Option<u64>,
        duration: Duration,
    ) {
        let labels = [
            ("operation", operation.to_string()),
            ("outcome", outcome.to_string()),
        ];

        metrics::counter!("download_operations_total", &labels).increment(1);
        metrics::histogram!("download_operation_duration_seconds", &labels)
            .record(duration.as_secs_f64());
        if let Some(size_bytes) = size_bytes {
            metrics::histogram!("download_operation_size_bytes", &labels).record(size_bytes as f64);
        }
    }

    pub fn record_billing_operation(&self, operation: &str, outcome: &str, duration: Duration) {
        let labels = [
            ("operation", operation.to_string()),
            ("outcome", outcome.to_string()),
        ];

        metrics::counter!("billing_operations_total", &labels).increment(1);
        metrics::histogram!("billing_operation_duration_seconds", &labels)
            .record(duration.as_secs_f64());
    }

    pub fn record_billing_webhook(
        &self,
        provider: &str,
        event_type: &str,
        outcome: &str,
        duration: Duration,
    ) {
        let labels = [
            ("provider", provider.to_string()),
            ("event_type", event_type.to_string()),
            ("outcome", outcome.to_string()),
        ];

        metrics::counter!("billing_webhooks_total", &labels).increment(1);
        metrics::histogram!("billing_webhook_duration_seconds", &labels)
            .record(duration.as_secs_f64());
    }

    pub fn record_worker_queue_job(&self, job_type: &str, outcome: &str, duration: Duration) {
        let labels = [
            ("job_type", job_type.to_string()),
            ("outcome", outcome.to_string()),
        ];

        metrics::counter!("worker_queue_jobs_total", &labels).increment(1);
        metrics::histogram!("worker_queue_job_duration_seconds", &labels)
            .record(duration.as_secs_f64());
    }

    pub fn record_worker_queue_recovery(&self, job_type: &str, outcome: &str) {
        let labels = [
            ("job_type", job_type.to_string()),
            ("outcome", outcome.to_string()),
        ];

        metrics::counter!("worker_queue_recovered_jobs_total", &labels).increment(1);
    }

    pub fn record_worker_queue_depth(
        &self,
        queue_name: &str,
        status: &str,
        depth: i64,
        oldest_age_seconds: Option<f64>,
    ) {
        let labels = [
            ("queue", queue_name.to_string()),
            ("status", status.to_string()),
        ];

        metrics::gauge!("worker_queue_depth", &labels).set(depth as f64);
        if let Some(oldest_age_seconds) = oldest_age_seconds {
            metrics::gauge!("worker_queue_oldest_age_seconds", &labels).set(oldest_age_seconds);
        }
    }

    pub fn record_object_storage_operation(
        &self,
        operation: &str,
        outcome: &str,
        object_count: u64,
        duration: Duration,
    ) {
        let labels = [
            ("operation", operation.to_string()),
            ("outcome", outcome.to_string()),
        ];

        metrics::counter!("object_storage_operations_total", &labels).increment(1);
        metrics::counter!("object_storage_objects_total", &labels).increment(object_count);
        metrics::histogram!("object_storage_operation_duration_seconds", &labels)
            .record(duration.as_secs_f64());
    }

    pub fn render(&self) -> String {
        self.handle.render()
    }
}

pub async fn metrics_handler(State(metrics): State<HttpMetrics>) -> impl IntoResponse {
    metrics.render()
}

pub async fn start_metrics_server(
    config: &AppConfig,
    metrics: HttpMetrics,
    bind_addr: &str,
) -> std::io::Result<JoinHandle<()>> {
    let listener = TcpListener::bind(bind_addr).await?;
    let addr = listener.local_addr()?;
    let router = Router::new()
        .route("/metrics", get(metrics_handler))
        .layer(middleware::from_fn_with_state(
            config.clone(),
            nvbes_core::http::internal_observability::internal_observability_guard,
        ))
        .with_state(metrics);

    tracing::info!(addr = %addr, "Starting worker metrics listener");

    Ok(tokio::spawn(async move {
        if let Err(error) = axum::serve(listener, router).await {
            tracing::error!(error = %error, "Worker metrics listener stopped");
        }
    }))
}

impl Default for HttpMetrics {
    fn default() -> Self {
        Self::new()
    }
}
