use nvbes_core::config::AppConfig;
use tracing_subscriber::prelude::*;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum OtlpProtocol {
    #[default]
    Grpc,
    Http,
}

#[derive(Debug, Clone, Copy)]
pub struct TracingConfig<'a> {
    pub environment: &'a str,
    pub otlp_endpoint: Option<&'a str>,
    pub otlp_authorization_header: Option<&'a str>,
    pub protocol: OtlpProtocol,
}

pub fn init_tracing(config: &AppConfig) {
    init_tracing_for_service(config, &config.app_name);
}

pub fn init_tracing_for_service(config: &AppConfig, service_name: &str) {
    init_tracing_with_config(
        TracingConfig {
            environment: &config.environment,
            otlp_endpoint: config.otlp_endpoint.as_deref(),
            otlp_authorization_header: config.otlp_authorization_header.as_deref(),
            protocol: OtlpProtocol::Grpc,
        },
        service_name,
    );
}

pub fn init_tracing_with_config(config: TracingConfig<'_>, service_name: &str) {
    let default_filter = "info";

    let env_filter = std::env::var("RUST_LOG")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .and_then(|value| tracing_subscriber::EnvFilter::try_new(value).ok())
        .unwrap_or_else(|| default_filter.into());

    let registry = tracing_subscriber::registry().with(env_filter);

    #[cfg(feature = "otlp")]
    let registry = registry.with(otlp_layer(config, service_name));

    if use_json_logs(config.environment) {
        registry
            .with(
                tracing_subscriber::fmt::layer()
                    .with_target(false)
                    .json()
                    .flatten_event(true),
            )
            .init();
    } else {
        registry
            .with(
                tracing_subscriber::fmt::layer()
                    .with_target(false)
                    .compact(),
            )
            .init();
    }
}

fn use_json_logs(environment: &str) -> bool {
    match std::env::var("NVBES_LOG_FORMAT").ok().as_deref() {
        Some("json") => true,
        Some("compact") => false,
        _ => environment != "development",
    }
}

#[cfg(feature = "otlp")]
fn otlp_layer<S>(
    config: TracingConfig<'_>,
    service_name: &str,
) -> Option<impl tracing_subscriber::Layer<S>>
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    let endpoint = config.otlp_endpoint?;

    use opentelemetry::KeyValue;
    use opentelemetry::trace::TracerProvider;
    use opentelemetry_otlp::{WithExportConfig, WithHttpConfig, WithTonicConfig};
    use opentelemetry_sdk::Resource;
    use opentelemetry_sdk::trace::SdkTracerProvider;

    let exporter = match config.protocol {
        OtlpProtocol::Grpc => {
            let mut exporter = opentelemetry_otlp::SpanExporter::builder()
                .with_tonic()
                .with_endpoint(endpoint.to_owned());
            if let Some(metadata) = otlp_metadata(config) {
                exporter = exporter.with_metadata(metadata);
            }
            exporter.build()
        }
        OtlpProtocol::Http => opentelemetry_otlp::SpanExporter::builder()
            .with_http()
            .with_endpoint(otlp_http_signal_endpoint(endpoint, "v1/traces"))
            .with_headers(otlp_headers(config))
            .build(),
    }
    .expect("failed to create OTLP exporter");

    let provider = SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(
            Resource::builder()
                .with_service_name(service_name.to_owned())
                .with_attributes([
                    KeyValue::new("service.namespace", "nvbes"),
                    KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
                    KeyValue::new("deployment.environment", config.environment.to_owned()),
                    KeyValue::new("deployment.environment.name", config.environment.to_owned()),
                ])
                .build(),
        )
        .build();

    // Set the global tracer provider so other parts of the app can use it.
    opentelemetry::global::set_tracer_provider(provider.clone());

    let tracer = provider.tracer("nvbes");
    Some(tracing_opentelemetry::layer().with_tracer(tracer))
}

pub fn otlp_http_signal_endpoint(base_endpoint: &str, signal_path: &str) -> String {
    format!(
        "{}/{}",
        base_endpoint.trim_end_matches('/'),
        signal_path.trim_start_matches('/')
    )
}

#[cfg(feature = "otlp")]
fn otlp_headers(config: TracingConfig<'_>) -> std::collections::HashMap<String, String> {
    config
        .otlp_authorization_header
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            std::collections::HashMap::from([("authorization".to_string(), value.to_string())])
        })
        .unwrap_or_default()
}

#[cfg(feature = "otlp")]
fn otlp_metadata(config: TracingConfig<'_>) -> Option<tonic::metadata::MetadataMap> {
    let authorization = config.otlp_authorization_header?.trim();
    if authorization.is_empty() {
        return None;
    }

    let value = tonic::metadata::MetadataValue::try_from(authorization)
        .expect("NVBES_OTLP_AUTHORIZATION_HEADER must be valid gRPC metadata");

    let mut metadata = tonic::metadata::MetadataMap::new();
    metadata.insert("authorization", value);
    Some(metadata)
}

#[cfg(test)]
mod tests {
    use super::otlp_http_signal_endpoint;

    #[test]
    fn http_signal_endpoint_is_canonical() {
        assert_eq!(
            otlp_http_signal_endpoint("https://example.grafana.net/otlp/", "/v1/traces"),
            "https://example.grafana.net/otlp/v1/traces"
        );
    }
}
