use sentry::protocol::Request;

use super::{is_safe_header_key, is_sensitive_key, scrub_sensitive_string};

pub(crate) fn scrub_request(request: &mut Request) {
    request.url = request.url.take().map(super::redaction::scrub_url);
    request.query_string = None;
    request.cookies = None;
    request.data = None;
    request.env.clear();
    request.headers.retain(|key, value| {
        if !is_safe_header_key(key) || is_sensitive_key(key) {
            return false;
        }
        *value = scrub_sensitive_string(value);
        true
    });
}
