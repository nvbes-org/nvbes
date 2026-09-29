use nvbes_observability::{OtlpProtocol, TracingConfig, init_tracing_with_config};

#[test]
fn installs_a_compact_subscriber_in_development() {
    unsafe { std::env::set_var("NVBES_LOG_FORMAT", "compact") };
    init_tracing_with_config(
        TracingConfig {
            environment: "development",
            otlp_endpoint: None,
            otlp_authorization_header: None,
            protocol: OtlpProtocol::Grpc,
        },
        "nvbes-observability-tracing-init",
    );
}
