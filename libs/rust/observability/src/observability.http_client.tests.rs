use axum::http::{Extensions, HeaderMap, HeaderValue};

use crate::trace_context::{TRACEPARENT_HEADER, TRACESTATE_HEADER, TraceParent, TraceStateValue};

use super::{propagate_headers_trace_context, propagate_trace_context, register_trace_context};

#[test]
fn register_trace_context_inserts_traceparent_extension() {
    let mut extensions = Extensions::new();
    register_trace_context(&mut extensions);
    assert!(extensions.get::<TraceParent>().is_some());
}

#[test]
fn propagate_trace_context_uses_registered_parent_and_tracestate() {
    let mut extensions = Extensions::new();
    let parent = TraceParent {
        trace_id: "4bf92f3577b34da6a3ce929d0e0e4736".into(),
        span_id: "00f067aa0ba902b7".into(),
        sampled: true,
    };
    extensions.insert(parent.clone());
    extensions.insert(TraceStateValue("congo=t61rcWkgMzE".into()));

    let mut headers = HeaderMap::new();
    propagate_trace_context(&extensions, &mut headers);

    assert_eq!(
        headers.get(TRACEPARENT_HEADER).unwrap().to_str().unwrap(),
        parent.to_header_value()
    );
    assert_eq!(
        headers.get(TRACESTATE_HEADER).unwrap().to_str().unwrap(),
        "congo=t61rcWkgMzE"
    );
}

#[test]
fn propagate_trace_context_generates_fresh_parent_when_missing() {
    let extensions = Extensions::new();
    let mut headers = HeaderMap::new();
    propagate_trace_context(&extensions, &mut headers);
    assert!(headers.contains_key(TRACEPARENT_HEADER));
}

#[test]
fn propagate_headers_trace_context_copies_inbound_context() {
    let mut request_headers = HeaderMap::new();
    request_headers.insert(
        TRACEPARENT_HEADER,
        HeaderValue::from_static("00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01"),
    );
    request_headers.insert(
        TRACESTATE_HEADER,
        HeaderValue::from_static("rojo=00f067aa0ba902b7"),
    );

    let mut outgoing = HeaderMap::new();
    propagate_headers_trace_context(&request_headers, &mut outgoing);
    assert!(outgoing.contains_key(TRACEPARENT_HEADER));
    assert_eq!(
        outgoing.get(TRACESTATE_HEADER).unwrap().to_str().unwrap(),
        "rojo=00f067aa0ba902b7"
    );
}

#[test]
fn propagate_headers_trace_context_falls_back_without_inbound() {
    let request_headers = HeaderMap::new();
    let mut outgoing = HeaderMap::new();
    propagate_headers_trace_context(&request_headers, &mut outgoing);
    assert!(outgoing.contains_key(TRACEPARENT_HEADER));
    assert!(!outgoing.contains_key(TRACESTATE_HEADER));
}
