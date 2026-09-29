#![cfg(feature = "otlp")]

use nvbes_observability::{OtlpProtocol, TracingConfig, init_tracing_with_config};

#[tokio::test]
async fn installs_json_logs_and_grpc_otlp_exporter() {
    unsafe { std::env::remove_var("NVBES_LOG_FORMAT") };
    unsafe { std::env::set_var("RUST_LOG", "info,nvbes_observability=debug") };
    init_tracing_with_config(
        TracingConfig {
            environment: "production",
            otlp_endpoint: Some("http://127.0.0.1:4317"),
            otlp_authorization_header: Some("Bearer coverage-token"),
            protocol: OtlpProtocol::Grpc,
        },
        "nvbes-observability-tracing-init",
    );
}
