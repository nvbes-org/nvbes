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

pub use error_reporting::{ErrorReportingGuard, init_error_reporting, install_safe_panic_hook};
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
pub use tracing::init_tracing;
pub use worker_error_reporting::{
    ErrorReportingSmokeResult, WorkerJobContext, WorkerMonitorSchedule,
    capture_error_reporting_smoke, capture_worker_heartbeat, capture_worker_job_error,
    start_worker_monitor_check_in, worker_monitor_slug,
};
