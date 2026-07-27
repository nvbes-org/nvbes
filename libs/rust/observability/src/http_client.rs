use axum::http::HeaderMap;

use crate::trace_context::{self, TraceParent, TraceStateValue};

/// Inject W3C Trace Context headers into an outgoing request's HeaderMap,
/// reading the incoming trace context from request Extensions.
///
/// Retrieves the current `TraceParent` from the request's Extensions (set by
/// `observe_request` middleware) and injects it as the parent context for the
/// downstream service.
///
/// Falls back to generating a fresh traceparent when no context is registered
/// (useful for background jobs, workers, or code outside the middleware).
pub fn propagate_trace_context(extensions: &axum::http::Extensions, headers: &mut HeaderMap) {
    let parent = extensions.get::<TraceParent>();
    let tracestate_snapshot = extensions.get::<TraceStateValue>();

    let outgoing = parent
        .cloned()
        .unwrap_or_else(|| trace_context::new_traceparent(true));

    trace_context::inject_traceparent_into(
        headers,
        &outgoing,
        tracestate_snapshot.map(|ts| ts.0.as_str()),
    );
}

/// Inject W3C Trace Context into outgoing headers, reading the incoming trace
/// context from a `HeaderMap` (as injected by `observe_request` middleware).
///
/// This variant is for handlers that receive `HeaderMap` directly rather than
/// `Request` with extensions. It parses `traceparent`/`tracestate` from
/// `request_headers` and injects that context into `outgoing_headers`.
pub fn propagate_headers_trace_context(
    request_headers: &HeaderMap,
    outgoing_headers: &mut HeaderMap,
) {
    let incoming = trace_context::extract_traceparent(request_headers);
    let tracestate = trace_context::extract_tracestate(request_headers);

    let outgoing = incoming.unwrap_or_else(|| trace_context::new_traceparent(true));

    trace_context::inject_traceparent_into(outgoing_headers, &outgoing, tracestate.as_deref());
}

/// Register a fresh `TraceParent` in an Extensions map so that downstream
/// propagation functions have context to work with.
///
/// Call this from contexts that do NOT go through the `observe_request`
/// middleware (workers, background jobs, tests) before making outgoing HTTP
/// calls.
pub fn register_trace_context(extensions: &mut axum::http::Extensions) {
    let tp = trace_context::new_traceparent(true);
    extensions.insert(tp);
}
