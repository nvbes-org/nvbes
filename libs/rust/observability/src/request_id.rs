use axum::{extract::Request, http::HeaderMap, http::header::HeaderName};
use uuid::Uuid;

pub const REQUEST_ID_HEADER: &str = "x-request-id";

pub fn request_id_header() -> HeaderName {
    HeaderName::from_static(REQUEST_ID_HEADER)
}

pub fn inbound_request_id(request: &Request) -> Option<String> {
    request_id_from_headers_optional(request.headers())
}

pub fn request_id_from_headers(headers: &HeaderMap) -> String {
    request_id_from_headers_optional(headers).unwrap_or_else(new_request_id)
}

fn request_id_from_headers_optional(headers: &HeaderMap) -> Option<String> {
    headers
        .get(REQUEST_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| is_safe_request_id(value))
        .map(ToOwned::to_owned)
}

pub fn new_request_id() -> String {
    format!("req_{}", Uuid::new_v4().simple())
}

pub fn is_safe_request_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 96
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

#[cfg(test)]
mod tests {
    use super::{REQUEST_ID_HEADER, is_safe_request_id, new_request_id, request_id_header};

    #[test]
    fn generated_request_ids_use_safe_prefix() {
        let request_id = new_request_id();

        assert!(request_id.starts_with("req_"));
        assert!(is_safe_request_id(&request_id));
    }

    #[test]
    fn unsafe_request_ids_are_rejected() {
        assert!(!is_safe_request_id(""));
        assert!(!is_safe_request_id("req with spaces"));
        assert!(!is_safe_request_id("req/with/slashes"));
        assert!(!is_safe_request_id(&"a".repeat(97)));
    }

    #[test]
    fn request_id_header_is_stable() {
        assert_eq!(request_id_header().as_str(), REQUEST_ID_HEADER);
    }
}
