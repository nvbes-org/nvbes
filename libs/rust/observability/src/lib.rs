pub mod error_reporting;
pub mod http_client;
pub mod metrics;
pub mod middleware;
#[cfg(feature = "profiling")]
pub mod profiling;
pub mod request_id;
pub mod trace_context;
pub mod tracing;
pub mod worker_error_reporting;

pub use error_reporting::{
    ErrorReportingConfig, ErrorReportingGuard, HttpServerErrorContext, capture_http_server_error,
    flush_error_reporting, init_error_reporting, init_error_reporting_for_service,
    init_error_reporting_with_config, install_safe_panic_hook, is_error_reporting_configured,
};
pub use http_client::{
    propagate_headers_trace_context, propagate_trace_context, register_trace_context,
};
pub use metrics::{metrics_handler, start_metrics_server};
#[cfg(feature = "profiling")]
pub use profiling::{ContinuousProfilingGuard, start_continuous_profiling};
pub use request_id::{
    inbound_request_id, is_safe_request_id, new_request_id, request_id_from_headers,
    request_id_header,
};
pub use trace_context::{
    TRACEPARENT_HEADER, TRACESTATE_HEADER, TraceParent, TraceStateValue, extract_traceparent,
    extract_tracestate, inject_traceparent_into, new_traceparent, parse_traceparent,
};
pub use tracing::{
    OtlpProtocol, TracingConfig, init_tracing, init_tracing_for_service,
    init_tracing_with_config, otlp_http_signal_endpoint,
};
pub use worker_error_reporting::{
    ErrorReportingSmokeResult, WorkerJobContext, WorkerMonitorSchedule, WorkerOperationContext,
    capture_error_reporting_smoke, capture_worker_heartbeat, capture_worker_job_error,
    capture_worker_operation_error, start_worker_monitor_check_in, worker_monitor_slug,
};
