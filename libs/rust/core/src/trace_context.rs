use axum::http::{HeaderMap, HeaderName, HeaderValue};
use reqwest::RequestBuilder;
use uuid::Uuid;

pub const TRACEPARENT_HEADER: &str = "traceparent";
pub const TRACESTATE_HEADER: &str = "tracestate";

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
}

#[derive(Debug, Clone)]
pub struct TraceStateValue(pub String);

pub fn traceparent_header_name() -> HeaderName {
    HeaderName::from_static(TRACEPARENT_HEADER)
}

pub fn tracestate_header_name() -> HeaderName {
    HeaderName::from_static(TRACESTATE_HEADER)
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

pub fn extract_traceparent(headers: &HeaderMap) -> Option<TraceParent> {
    headers
        .get(TRACEPARENT_HEADER)
        .and_then(|value| value.to_str().ok())
        .and_then(parse_traceparent)
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

    if let Some(tracestate) = tracestate
        && let Ok(header_value) = HeaderValue::from_str(tracestate)
    {
        headers.insert(tracestate_header_name(), header_value);
    }
}

pub fn trace_headers(traceparent: &TraceParent, tracestate: Option<&str>) -> HeaderMap {
    let mut headers = HeaderMap::new();
    inject_traceparent_into(&mut headers, traceparent, tracestate);
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
    fn parse_valid_traceparent_unsampled() {
        let result =
            parse_traceparent("00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-00").unwrap();
        assert!(!result.sampled);
        assert_eq!(
            result.to_header_value(),
            "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-00"
        );
    }

    #[test]
    fn parse_traceparent_table_rejects_invalid_shapes() {
        let cases = [
            "",
            "01-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01",
            "00-SHORT-b7ad6b7169203331-01",
            "00-0AF7651916CD43DD8448EB211C80319C-b7ad6b7169203331-01",
            "00-00000000000000000000000000000000-b7ad6b7169203331-01",
            "00-0af7651916cd43dd8448eb211c80319c-SHORT-01",
            "00-0af7651916cd43dd8448eb211c80319c-B7AD6B7169203331-01",
            "00-0af7651916cd43dd8448eb211c80319c-0000000000000000-01",
            "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-0",
            "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-0G",
            "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331",
        ];
        for value in cases {
            assert!(
                parse_traceparent(value).is_none(),
                "expected reject for {value:?}"
            );
        }
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
    fn extract_and_inject_round_trip_with_tracestate() {
        let parent = new_traceparent(true);
        let mut headers = HeaderMap::new();
        inject_traceparent_into(&mut headers, &parent, Some("vendor=value"));

        assert_eq!(
            headers
                .get(traceparent_header_name())
                .and_then(|v| v.to_str().ok()),
            Some(parent.to_header_value().as_str())
        );
        assert_eq!(
            extract_tracestate(&headers).as_deref(),
            Some("vendor=value")
        );
        assert_eq!(extract_traceparent(&headers), Some(parent.clone()));

        let rebuilt = trace_headers(&parent, Some("vendor=value"));
        assert_eq!(extract_traceparent(&rebuilt), Some(parent));
        assert_eq!(
            extract_tracestate(&rebuilt).as_deref(),
            Some("vendor=value")
        );
    }

    #[test]
    fn extract_trace_headers_return_none_when_absent_or_invalid() {
        let empty = HeaderMap::new();
        assert!(extract_traceparent(&empty).is_none());
        assert!(extract_tracestate(&empty).is_none());

        let mut headers = HeaderMap::new();
        headers.insert(
            TRACEPARENT_HEADER,
            HeaderValue::from_static("not-a-traceparent"),
        );
        assert!(extract_traceparent(&headers).is_none());
    }

    #[test]
    fn fresh_trace_headers_include_w3c_headers() {
        let headers = fresh_trace_headers(true);

        assert!(headers.get(TRACEPARENT_HEADER).is_some());
        assert!(extract_traceparent(&headers).is_some());
        assert_eq!(tracestate_header_name().as_str(), TRACESTATE_HEADER);
        let _ = TraceStateValue("cong=1".into());
    }

    #[test]
    fn inject_without_tracestate_leaves_tracestate_absent() {
        let parent = new_traceparent(false);
        let headers = trace_headers(&parent, None);
        assert!(headers.get(TRACESTATE_HEADER).is_none());
        assert!(!extract_traceparent(&headers).unwrap().sampled);
    }

    #[test]
    fn with_fresh_trace_headers_attaches_traceparent() {
        let client = reqwest::Client::new();
        let request = with_fresh_trace_headers(client.get("http://127.0.0.1/health"))
            .build()
            .expect("request builds");
        assert!(request.headers().get(TRACEPARENT_HEADER).is_some());
    }
}

#[cfg(test)]
#[path = "trace_context.property.tests.rs"]
mod property_tests;
