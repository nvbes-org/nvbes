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
#[path = "observability.request_id.tests.rs"]
mod tests;
