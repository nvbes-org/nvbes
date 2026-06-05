use axum::http::{HeaderMap, HeaderName, HeaderValue};
use uuid::Uuid;

pub const TRACEPARENT_HEADER: &str = "traceparent";
pub const TRACESTATE_HEADER: &str = "tracestate";
pub const SENTRY_TRACE_HEADER: &str = "sentry-trace";

pub fn traceparent_header_name() -> HeaderName {
    HeaderName::from_static(TRACEPARENT_HEADER)
}

pub fn tracestate_header_name() -> HeaderName {
    HeaderName::from_static(TRACESTATE_HEADER)
}

pub fn sentry_trace_header_name() -> HeaderName {
    HeaderName::from_static(SENTRY_TRACE_HEADER)
}

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

/// Generate a child traceparent — same trace_id, new span_id.
pub fn child_traceparent(parent: &TraceParent) -> TraceParent {
    TraceParent {
        trace_id: parent.trace_id.clone(),
        span_id: new_span_id(),
        sampled: parent.sampled,
    }
}

/// Parse a W3C `traceparent` header value.
/// Format: `{version}-{trace_id}-{span_id}-{trace_flags}`
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
    let sampled = flag_byte & TRACE_FLAG_SAMPLED != 0;

    Some(TraceParent {
        trace_id: trace_id.to_string(),
        span_id: span_id.to_string(),
        sampled,
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

/// Extract an incoming `traceparent` from request headers.
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

/// Extract `tracestate` from request headers, returning the raw value.
pub fn extract_tracestate(headers: &HeaderMap) -> Option<String> {
    headers
        .get(TRACESTATE_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned)
}

/// Inject `traceparent` (and optionally `tracestate`) into a HeaderMap for
/// outgoing requests. Uses the provided `TraceParent` as the source.
pub fn inject_traceparent_into(
    headers: &mut HeaderMap,
    tp: &TraceParent,
    tracestate: Option<&str>,
) {
    let value = tp.to_header_value();
    if let Ok(header_value) = HeaderValue::from_str(&value) {
        headers.insert(HeaderName::from_static(TRACEPARENT_HEADER), header_value);
    }
    if let Some(ts) = tracestate {
        if let Ok(header_value) = HeaderValue::from_str(ts) {
            headers.insert(HeaderName::from_static(TRACESTATE_HEADER), header_value);
        }
    }
}

pub fn inject_sentry_trace_into(headers: &mut HeaderMap, tp: &TraceParent) {
    let value = tp.to_sentry_trace_header_value();
    if let Ok(header_value) = HeaderValue::from_str(&value) {
        headers.insert(HeaderName::from_static(SENTRY_TRACE_HEADER), header_value);
    }
}

/// Wrapper newtype to store `tracestate` in `axum::http::Extensions` without
/// colliding with other `String` values.
#[derive(Debug, Clone)]
pub struct TraceStateValue(pub String);

fn is_lower_hex(s: &str) -> bool {
    s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_traceparent_sampled() {
        let result = parse_traceparent("00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01");
        assert!(result.is_some());
        let tp = result.unwrap();
        assert_eq!(tp.trace_id, "0af7651916cd43dd8448eb211c80319c");
        assert_eq!(tp.span_id, "b7ad6b7169203331");
        assert!(tp.sampled);
    }

    #[test]
    fn parse_valid_traceparent_not_sampled() {
        let result = parse_traceparent("00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-00");
        assert!(result.is_some());
        assert!(!result.unwrap().sampled);
    }

    #[test]
    fn parse_valid_sentry_trace_sampled() {
        let result = parse_sentry_trace("0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-1");
        assert!(result.is_some());
        let tp = result.unwrap();
        assert_eq!(tp.trace_id, "0af7651916cd43dd8448eb211c80319c");
        assert_eq!(tp.span_id, "b7ad6b7169203331");
        assert!(tp.sampled);
    }

    #[test]
    fn parse_valid_sentry_trace_defaults_unknown_sampling_to_sampled() {
        let result = parse_sentry_trace("0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331");
        assert!(result.is_some());
        assert!(result.unwrap().sampled);
    }

    #[test]
    fn reject_all_zero_trace_id() {
        assert!(
            parse_traceparent("00-00000000000000000000000000000000-b7ad6b7169203331-01").is_none()
        );
    }

    #[test]
    fn reject_all_zero_span_id() {
        assert!(
            parse_traceparent("00-0af7651916cd43dd8448eb211c80319c-0000000000000000-01").is_none()
        );
    }

    #[test]
    fn reject_invalid_version() {
        assert!(
            parse_traceparent("ff-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01").is_none()
        );
    }

    #[test]
    fn reject_invalid_hex_trace_id() {
        assert!(
            parse_traceparent("00-zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz-b7ad6b7169203331-01").is_none()
        );
    }

    #[test]
    fn child_keeps_same_trace_id() {
        let parent =
            parse_traceparent("00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01").unwrap();
        let child = child_traceparent(&parent);
        assert_eq!(child.trace_id, parent.trace_id);
        assert_ne!(child.span_id, parent.span_id);
        assert_eq!(child.sampled, parent.sampled);
        assert_eq!(child.span_id.len(), SPAN_ID_LEN);
    }

    #[test]
    fn new_traceparent_is_valid() {
        let tp = new_traceparent(true);
        let serialized = tp.to_header_value();
        let reparsed = parse_traceparent(&serialized);
        assert!(reparsed.is_some());
        assert_eq!(reparsed.unwrap(), tp);
    }

    #[test]
    fn to_header_value_format() {
        let tp = TraceParent {
            trace_id: "0af7651916cd43dd8448eb211c80319c".to_string(),
            span_id: "b7ad6b7169203331".to_string(),
            sampled: true,
        };
        assert_eq!(
            tp.to_header_value(),
            "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01"
        );
        assert_eq!(
            tp.to_sentry_trace_header_value(),
            "0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-1"
        );
    }

    #[test]
    fn inject_and_extract() {
        let tp = new_traceparent(true);
        let mut headers = HeaderMap::new();
        inject_traceparent_into(&mut headers, &tp, Some("foo=bar"));
        let extracted = extract_traceparent(&headers);
        assert!(extracted.is_some());
        assert_eq!(extracted.unwrap(), tp);
        assert_eq!(extract_tracestate(&headers).as_deref(), Some("foo=bar"));
    }

    #[test]
    fn inject_and_extract_sentry_trace() {
        let tp = new_traceparent(true);
        let mut headers = HeaderMap::new();
        inject_sentry_trace_into(&mut headers, &tp);
        let extracted = extract_sentry_trace(&headers);
        assert!(extracted.is_some());
        assert_eq!(extracted.unwrap(), tp);
    }
}
