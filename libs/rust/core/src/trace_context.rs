use axum::http::{HeaderMap, HeaderName, HeaderValue};
use reqwest::RequestBuilder;
use uuid::Uuid;

pub const TRACEPARENT_HEADER: &str = "traceparent";
pub const TRACESTATE_HEADER: &str = "tracestate";
pub const SENTRY_TRACE_HEADER: &str = "sentry-trace";

const TRACEPARENT_VERSION: &str = "00";
const TRACE_FLAG_SAMPLED: u8 = 0x01;
const TRACE_ID_LEN: usize = 32;
const SPAN_ID_LEN: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceParent {
    pub trace_id: String,
    pub span_id: String,
    pub sampled: bool,
}

impl TraceParent {
    pub fn to_header_value(&self) -> String {
        let flags = if self.sampled { "01" } else { "00" };
        format!(
            "{}-{}-{}-{}",
            TRACEPARENT_VERSION, self.trace_id, self.span_id, flags
        )
    }

    pub fn to_sentry_trace_header_value(&self) -> String {
        let sampled = if self.sampled { "1" } else { "0" };
        format!("{}-{}-{}", self.trace_id, self.span_id, sampled)
    }
}

#[derive(Debug, Clone)]
pub struct TraceStateValue(pub String);

pub fn traceparent_header_name() -> HeaderName {
    HeaderName::from_static(TRACEPARENT_HEADER)
}

pub fn tracestate_header_name() -> HeaderName {
    HeaderName::from_static(TRACESTATE_HEADER)
}

pub fn sentry_trace_header_name() -> HeaderName {
    HeaderName::from_static(SENTRY_TRACE_HEADER)
}

pub fn new_trace_id() -> String {
    Uuid::new_v4().simple().to_string()
}

pub fn new_span_id() -> String {
    Uuid::new_v4().simple().to_string()[..SPAN_ID_LEN].to_string()
}

pub fn new_traceparent(sampled: bool) -> TraceParent {
    TraceParent {
        trace_id: new_trace_id(),
        span_id: new_span_id(),
        sampled,
    }
}

pub fn child_traceparent(parent: &TraceParent) -> TraceParent {
    TraceParent {
        trace_id: parent.trace_id.clone(),
        span_id: new_span_id(),
        sampled: parent.sampled,
    }
}

pub fn parse_traceparent(value: &str) -> Option<TraceParent> {
    let mut parts = value.splitn(4, '-');
    let version = parts.next()?;
    let trace_id = parts.next()?;
    let span_id = parts.next()?;
    let flags = parts.next()?;

    if version != TRACEPARENT_VERSION {
        return None;
    }
    if trace_id.len() != TRACE_ID_LEN || !is_lower_hex(trace_id) {
        return None;
    }
    if trace_id.chars().all(|c| c == '0') {
        return None;
    }
    if span_id.len() != SPAN_ID_LEN || !is_lower_hex(span_id) {
        return None;
    }
    if span_id.chars().all(|c| c == '0') {
        return None;
    }
    if flags.len() != 2 || !is_lower_hex(flags) {
        return None;
    }

    let flag_byte = u8::from_str_radix(flags, 16).ok()?;
    Some(TraceParent {
        trace_id: trace_id.to_string(),
        span_id: span_id.to_string(),
        sampled: flag_byte & TRACE_FLAG_SAMPLED != 0,
    })
}

pub fn parse_sentry_trace(value: &str) -> Option<TraceParent> {
    let mut parts = value.trim().splitn(3, '-');
    let trace_id = parts.next()?;
    let span_id = parts.next()?;
    let sampled = parts.next().map(|value| match value {
        "1" => Some(true),
        "0" => Some(false),
        _ => None,
    });

    if trace_id.len() != TRACE_ID_LEN || !is_lower_hex(trace_id) {
        return None;
    }
    if trace_id.chars().all(|c| c == '0') {
        return None;
    }
    if span_id.len() != SPAN_ID_LEN || !is_lower_hex(span_id) {
        return None;
    }
    if span_id.chars().all(|c| c == '0') {
        return None;
    }

    Some(TraceParent {
        trace_id: trace_id.to_string(),
        span_id: span_id.to_string(),
        sampled: sampled.flatten().unwrap_or(true),
    })
}

pub fn extract_traceparent(headers: &HeaderMap) -> Option<TraceParent> {
    headers
        .get(TRACEPARENT_HEADER)
        .and_then(|value| value.to_str().ok())
        .and_then(parse_traceparent)
}

pub fn extract_sentry_trace(headers: &HeaderMap) -> Option<TraceParent> {
    headers
        .get(SENTRY_TRACE_HEADER)
        .and_then(|value| value.to_str().ok())
        .and_then(parse_sentry_trace)
}

pub fn extract_tracestate(headers: &HeaderMap) -> Option<String> {
    headers
        .get(TRACESTATE_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned)
}

pub fn inject_traceparent_into(
    headers: &mut HeaderMap,
    traceparent: &TraceParent,
    tracestate: Option<&str>,
) {
    if let Ok(header_value) = HeaderValue::from_str(&traceparent.to_header_value()) {
        headers.insert(traceparent_header_name(), header_value);
    }

    if let Some(tracestate) = tracestate {
        if let Ok(header_value) = HeaderValue::from_str(tracestate) {
            headers.insert(tracestate_header_name(), header_value);
        }
    }
}

pub fn inject_sentry_trace_into(headers: &mut HeaderMap, traceparent: &TraceParent) {
    if let Ok(header_value) = HeaderValue::from_str(&traceparent.to_sentry_trace_header_value()) {
        headers.insert(sentry_trace_header_name(), header_value);
    }
}

pub fn trace_headers(traceparent: &TraceParent, tracestate: Option<&str>) -> HeaderMap {
    let mut headers = HeaderMap::new();
    inject_traceparent_into(&mut headers, traceparent, tracestate);
    inject_sentry_trace_into(&mut headers, traceparent);
    headers
}

pub fn fresh_trace_headers(sampled: bool) -> HeaderMap {
    trace_headers(&new_traceparent(sampled), None)
}

pub fn with_fresh_trace_headers(request: RequestBuilder) -> RequestBuilder {
    request.headers(fresh_trace_headers(true))
}

fn is_lower_hex(value: &str) -> bool {
    value
        .bytes()
        .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_traceparent_sampled() {
        let result = parse_traceparent("00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01");
        assert!(result.is_some());
        assert!(result.unwrap().sampled);
    }

    #[test]
    fn parse_valid_sentry_trace_without_sampling_defaults_to_sampled() {
        let result = parse_sentry_trace("0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331");
        assert!(result.is_some());
        assert!(result.unwrap().sampled);
    }

    #[test]
    fn child_keeps_trace_id_and_changes_span_id() {
        let parent =
            parse_traceparent("00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01").unwrap();
        let child = child_traceparent(&parent);

        assert_eq!(child.trace_id, parent.trace_id);
        assert_ne!(child.span_id, parent.span_id);
        assert_eq!(child.sampled, parent.sampled);
    }

    #[test]
    fn fresh_trace_headers_include_w3c_and_sentry_headers() {
        let headers = fresh_trace_headers(true);

        assert!(headers.get(TRACEPARENT_HEADER).is_some());
        assert!(headers.get(SENTRY_TRACE_HEADER).is_some());
        assert!(extract_traceparent(&headers).is_some());
        assert!(extract_sentry_trace(&headers).is_some());
    }
}
