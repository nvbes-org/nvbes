#![cfg(feature = "otlp")]

use nvbes_observability::{OtlpProtocol, TracingConfig, init_tracing_with_config};

#[tokio::test]
async fn installs_http_otlp_exporter_with_authorization_headers() {
    init_tracing_with_config(
        TracingConfig {
            environment: "staging",
            otlp_endpoint: Some("https://otlp.example.test/otlp/"),
            otlp_authorization_header: Some("Basic dXNlcjpwYXNz"),
            protocol: OtlpProtocol::Http,
        },
        "nvbes-observability-tracing-init-http",
    );
}
