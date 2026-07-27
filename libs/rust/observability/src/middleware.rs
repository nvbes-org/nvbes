use axum::{
    extract::{MatchedPath, Request, State},
    http::{HeaderName, HeaderValue},
    middleware::Next,
    response::Response,
};
use std::time::Instant;

use crate::metrics::HttpMetrics;
#[cfg(feature = "otlp")]
use crate::trace_context::TraceParent;
use crate::trace_context::{self, TraceStateValue};
use crate::{inbound_request_id, new_request_id, request_id_header};

const SERVER_TIMING_HEADER: HeaderName = HeaderName::from_static("server-timing");

pub async fn observe_request(
    State(metrics): State<HttpMetrics>,
    req: Request,
    next: Next,
) -> Response {
    let mut req = req;
    let started_at = Instant::now();
    let method = req.method().clone();
    let path_template = request_path_template(
        req.extensions()
            .get::<MatchedPath>()
            .map(|matched_path| matched_path.as_str()),
    );

    // --- W3C Trace Context ---
    let incoming_traceparent = trace_context::extract_traceparent(req.headers());
    let tracestate = trace_context::extract_tracestate(req.headers());
    let mut current_traceparent = incoming_traceparent
        .as_ref()
        .map(trace_context::child_traceparent)
        .unwrap_or_else(|| trace_context::new_traceparent(true));

    #[cfg(feature = "otlp")]
    let otel_span = build_otel_span(&incoming_traceparent, &method, &path_template);
    #[cfg(feature = "otlp")]
    if let Some(traceparent) = otel_span.as_ref().and_then(traceparent_from_span) {
        current_traceparent = traceparent;
    }
    #[cfg(feature = "otlp")]
    let _span_guard = otel_span.as_ref().map(|s| s.enter());

    // Store trace context in extensions so downstream handlers and outgoing
    // HTTP clients can retrieve it for propagation.
    req.extensions_mut().insert(current_traceparent.clone());
    if let Some(ref ts) = tracestate {
        req.extensions_mut().insert(TraceStateValue(ts.clone()));
    }

    // Also inject traceparent into request headers so that handlers using
    // HeaderMap extractors (without Request) can still propagate it.
    if let Ok(value) = axum::http::HeaderValue::from_str(&current_traceparent.to_header_value()) {
        req.headers_mut()
            .insert(trace_context::traceparent_header_name(), value);
    }
    if let Some(ref ts) = tracestate
        && let Ok(value) = axum::http::HeaderValue::from_str(ts)
    {
        req.headers_mut()
            .insert(trace_context::tracestate_header_name(), value);
    }

    // --- Request ID (keep backward compat) ---
    let request_id = inbound_request_id(&req).unwrap_or_else(new_request_id);

    req.extensions_mut().insert(request_id.clone());
    metrics.start_request();

    tracing::info!(
        target: "nvbes::technical_log",
        event_category = "technical_log",
        event_name = "http.request_started",
        request_id = %request_id,
        trace_id = %current_traceparent.trace_id,
        span_id = %current_traceparent.span_id,
        method = %method,
        path_template = %path_template,
    );

    let mut response = next.run(req).await;
    let status = response.status();
    let duration_ms = started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;

    metrics.finish_request(
        method.as_str(),
        &path_template,
        status.as_u16(),
        started_at.elapsed(),
    );

    #[cfg(feature = "otlp")]
    if let Some(span) = &otel_span {
        span.record("http.response.status_code", status.as_u16());
        if status.is_server_error() {
            span.record("otel.status_code", "ERROR");
        }
    }

    // --- Response headers ---
    if let Ok(value) = HeaderValue::from_str(&request_id) {
        response.headers_mut().insert(request_id_header(), value);
    }
    if let Ok(value) = HeaderValue::from_str(&current_traceparent.to_header_value()) {
        response
            .headers_mut()
            .insert(trace_context::traceparent_header_name(), value);
    }
    if let Some(ts) = &tracestate
        && let Ok(value) = HeaderValue::from_str(ts)
    {
        response
            .headers_mut()
            .insert(trace_context::tracestate_header_name(), value);
    }
    if let Ok(value) = HeaderValue::from_str(&server_timing_value(duration_ms)) {
        response.headers_mut().insert(SERVER_TIMING_HEADER, value);
    }

    // --- Structured logging with both request_id and trace context ---
    let trace_id = &current_traceparent.trace_id;
    let span_id = &current_traceparent.span_id;

    if status.is_server_error() {
        tracing::error!(
            target: "nvbes::technical_log",
            event_category = "technical_log",
            event_name = "http.request",
            request_id = %request_id,
            trace_id = %trace_id,
            span_id = %span_id,
            method = %method,
            path_template = %path_template,
            status = status.as_u16(),
            duration_ms,
        );
        crate::error_reporting::capture_http_server_error(
            &crate::error_reporting::HttpServerErrorContext {
                method: method.as_str(),
                path_template: &path_template,
                status: status.as_u16(),
                request_id: &request_id,
                trace_id,
                span_id,
                duration_ms,
            },
        );
    } else if status.is_client_error() {
        tracing::warn!(
            target: "nvbes::technical_log",
            event_category = "technical_log",
            event_name = "http.request",
            request_id = %request_id,
            trace_id = %trace_id,
            span_id = %span_id,
            method = %method,
            path_template = %path_template,
            status = status.as_u16(),
            duration_ms,
        );
    } else {
        tracing::info!(
            target: "nvbes::technical_log",
            event_category = "technical_log",
            event_name = "http.request",
            request_id = %request_id,
            trace_id = %trace_id,
            span_id = %span_id,
            method = %method,
            path_template = %path_template,
            status = status.as_u16(),
            duration_ms,
        );
    }

    #[cfg(feature = "otlp")]
    if let Some(guard) = _span_guard {
        drop(guard);
    }

    response
}

#[cfg(feature = "otlp")]
fn build_otel_span(
    incoming: &Option<TraceParent>,
    method: &axum::http::Method,
    path_template: &str,
) -> Option<tracing::Span> {
    use opentelemetry::trace::{
        SpanContext, SpanId, TraceContextExt, TraceFlags, TraceId, TraceState,
    };
    use tracing_opentelemetry::OpenTelemetrySpanExt;

    let span = tracing::info_span!(
        "HTTP request",
        otel.name = %format!("{} {}", method, path_template),
        otel.kind = "server",
        http.request.method = %method,
        http.route = %path_template,
        http.response.status_code = tracing::field::Empty,
        otel.status_code = tracing::field::Empty,
    );

    if let Some(parent) = incoming
        && let Ok(trace_id) = TraceId::from_hex(&parent.trace_id)
        && let Ok(parent_span_id) = SpanId::from_hex(&parent.span_id)
    {
        let trace_flags = if parent.sampled {
            TraceFlags::SAMPLED
        } else {
            TraceFlags::default()
        };
        let parent_ctx = SpanContext::new(
            trace_id,
            parent_span_id,
            trace_flags,
            true,
            TraceState::default(),
        );
        let cx = opentelemetry::Context::new().with_remote_span_context(parent_ctx);
        span.set_parent(cx).ok();
    }

    Some(span)
}

#[cfg(feature = "otlp")]
fn traceparent_from_span(span: &tracing::Span) -> Option<TraceParent> {
    use opentelemetry::trace::TraceContextExt;
    use tracing_opentelemetry::OpenTelemetrySpanExt;

    let context = span.context();
    let span = context.span();
    let span_context = span.span_context();
    span_context.is_valid().then(|| TraceParent {
        trace_id: span_context.trace_id().to_string(),
        span_id: span_context.span_id().to_string(),
        sampled: span_context.is_sampled(),
    })
}

fn request_path_template(path_template: Option<&str>) -> String {
    path_template
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| "unmatched".to_owned())
}

fn server_timing_value(duration_ms: u64) -> String {
    format!("app;dur={duration_ms}")
}

#[cfg(test)]
mod tests {
    use super::{request_path_template, server_timing_value};
    use axum::{body::Body, http::Request};

    #[test]
    fn request_path_template_uses_matched_path_when_present() {
        let template = request_path_template(Some("/files/{file_id}"));

        assert_eq!(template, "/files/{file_id}");
    }

    #[test]
    fn request_path_template_falls_back_when_unmatched() {
        let _req = Request::builder()
            .uri("/files/123")
            .body(Body::empty())
            .expect("request");

        assert_eq!(request_path_template(None), "unmatched");
    }

    #[test]
    fn server_timing_reports_total_app_duration() {
        assert_eq!(server_timing_value(42), "app;dur=42");
    }
}
