use axum::{
    body::Body,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use sha2::{Digest, Sha256};

const CONTENT_DIGEST: &str = "content-digest";
const SHA256_ALGO: &str = "sha-256";

pub async fn content_digest_guard(
    headers: HeaderMap,
    request: axum::http::Request<Body>,
    next: Next,
) -> Result<Response, impl IntoResponse> {
    let digest_header = match headers.get(CONTENT_DIGEST) {
        Some(v) => v,
        None => return Ok(next.run(request).await),
    };

    let header_str = match digest_header.to_str() {
        Ok(s) => s,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                "Invalid Content-Digest header encoding",
            ));
        }
    };

    let expected = match parse_sha256_digest(header_str) {
        Some(hash) => hash,
        None => return Ok(next.run(request).await),
    };

    const MAX_BUFFERED_BODY: usize = 10 * 1024 * 1024;

    let (parts, body) = request.into_parts();
    let body_bytes = match axum::body::to_bytes(body, MAX_BUFFERED_BODY).await {
        Ok(b) => b,
        Err(_) => {
            return Err((StatusCode::BAD_REQUEST, "Request body too large"));
        }
    };

    let actual = sha256_digest_base64(&body_bytes);

    if actual != expected {
        return Err((
            StatusCode::BAD_REQUEST,
            "Content-Digest mismatch: request body does not match the provided digest",
        ));
    }

    let request = axum::http::Request::from_parts(parts, Body::from(body_bytes));
    Ok(next.run(request).await)
}

pub(crate) fn parse_sha256_digest(header_value: &str) -> Option<String> {
    let value = header_value.trim();

    let stripped = value.strip_prefix(SHA256_ALGO)?.strip_prefix('=')?;
    let stripped = stripped.trim();

    let inner = stripped
        .strip_prefix(':')
        .and_then(|s| s.strip_suffix(':'))
        .unwrap_or(stripped);

    // Validate it looks like base64 (no colons, no whitespace in the hash part)
    if inner.is_empty() || inner.contains(':') || inner.contains(char::is_whitespace) {
        return None;
    }

    Some(inner.to_string())
}

pub fn sha256_digest_base64(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let hash = hasher.finalize();
    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, hash)
}

pub fn content_digest_header_value(bytes: &[u8]) -> String {
    format!("{SHA256_ALGO}=:{}:", sha256_digest_base64(bytes))
}

#[cfg(test)]
#[path = "http.content_digest.tests.rs"]
mod tests;

#[cfg(test)]
#[path = "http.content_digest.property.tests.rs"]
mod property_tests;
