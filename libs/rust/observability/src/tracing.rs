use nvbes_core::config::AppConfig;
use tracing_subscriber::prelude::*;

pub fn init_tracing(config: &AppConfig) {
    let default_filter = "info";

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| default_filter.into());

    let registry = tracing_subscriber::registry().with(env_filter);

    #[cfg(feature = "otlp")]
    let registry = registry.with(otlp_layer(config));

    if config.environment == "development" {
        registry
            .with(
                tracing_subscriber::fmt::layer()
                    .with_target(false)
                    .compact(),
            )
            .init();
    } else {
        registry
            .with(
                tracing_subscriber::fmt::layer()
                    .with_target(false)
                    .json()
                    .flatten_event(true),
            )
            .init();
    }
}

#[cfg(feature = "otlp")]
fn otlp_layer<S>(config: &AppConfig) -> Option<impl tracing_subscriber::Layer<S>>
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    let endpoint = config.otlp_endpoint.as_ref()?;

    use opentelemetry::trace::TracerProvider;
    use opentelemetry_otlp::{WithExportConfig, WithTonicConfig};
    use opentelemetry_sdk::Resource;
    use opentelemetry_sdk::trace::SdkTracerProvider;

    let mut exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint.clone());

    if let Some(metadata) = otlp_metadata(config) {
        exporter = exporter.with_metadata(metadata);
    }

    let exporter = exporter.build().expect("failed to create OTLP exporter");

    let provider = SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(
            Resource::builder()
                .with_service_name(config.app_name.clone())
                .build(),
        )
        .build();

    // Set the global tracer provider so other parts of the app can use it.
    opentelemetry::global::set_tracer_provider(provider.clone());

    let tracer = provider.tracer("nvbes");
    Some(tracing_opentelemetry::layer().with_tracer(tracer))
}

#[cfg(feature = "otlp")]
fn otlp_metadata(config: &AppConfig) -> Option<tonic::metadata::MetadataMap> {
    let authorization = config.otlp_authorization_header.as_deref()?.trim();
    if authorization.is_empty() {
        return None;
    }

    let value = tonic::metadata::MetadataValue::try_from(authorization)
        .expect("NVBES_OTLP_AUTHORIZATION_HEADER must be valid gRPC metadata");

    let mut metadata = tonic::metadata::MetadataMap::new();
    metadata.insert("authorization", value);
    Some(metadata)
}
