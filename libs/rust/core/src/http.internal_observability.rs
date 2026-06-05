use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::config::AppConfig;

const INTERNAL_TOKEN_HEADER: &str = "x-nvbes-internal-token";
const BEARER_PREFIX: &str = "Bearer ";

pub async fn internal_observability_guard(
    State(config): State<AppConfig>,
    headers: HeaderMap,
    request: axum::http::Request<Body>,
    next: Next,
) -> Result<Response, impl IntoResponse> {
    let Some(expected_token) = config.observability_internal_token.as_deref() else {
        if config.environment == "development" {
            return Ok(next.run(request).await);
        }

        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            "Internal observability authentication is not configured",
        ));
    };

    if internal_token_matches(&headers, expected_token) {
        return Ok(next.run(request).await);
    }

    Err((
        StatusCode::UNAUTHORIZED,
        "Internal observability authentication required",
    ))
}

fn internal_token_matches(headers: &HeaderMap, expected_token: &str) -> bool {
    let header_token = headers
        .get(INTERNAL_TOKEN_HEADER)
        .and_then(|value| value.to_str().ok());
    let bearer_token = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix(BEARER_PREFIX));

    header_token
        .into_iter()
        .chain(bearer_token)
        .any(|token| constant_time_eq(token.as_bytes(), expected_token.as_bytes()))
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }

    let mut diff = 0u8;
    for (left_byte, right_byte) in left.iter().zip(right.iter()) {
        diff |= left_byte ^ right_byte;
    }

    diff == 0
}

#[cfg(test)]
mod tests {
    use super::{INTERNAL_TOKEN_HEADER, internal_token_matches};
    use axum::http::{HeaderMap, HeaderValue, header};

    const TOKEN: &str = "internal-observability-token-32b";

    #[test]
    fn internal_token_matches_custom_header() {
        let mut headers = HeaderMap::new();
        headers.insert(INTERNAL_TOKEN_HEADER, HeaderValue::from_static(TOKEN));

        assert!(internal_token_matches(&headers, TOKEN));
    }

    #[test]
    fn internal_token_matches_bearer_header() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer internal-observability-token-32b"),
        );

        assert!(internal_token_matches(&headers, TOKEN));
    }

    #[test]
    fn internal_token_rejects_wrong_value() {
        let mut headers = HeaderMap::new();
        headers.insert(
            INTERNAL_TOKEN_HEADER,
            HeaderValue::from_static("wrong-internal-observability-token"),
        );

        assert!(!internal_token_matches(&headers, TOKEN));
    }
}
