use axum::{
    extract::State,
    http::{HeaderMap, Method, Request, header},
    middleware::Next,
    response::Response,
};

use crate::{app::AppState, http::error::AppError};

const ORIGIN_SKIP_PATHS: &[&str] = &[
    "/oauth/token",
    "/oauth/introspect",
    "/oauth/revoke",
    "/oauth/par",
];

pub async fn origin_guard(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, AppError> {
    if !is_mutating_method(request.method()) {
        return Ok(next.run(request).await);
    }

    if ORIGIN_SKIP_PATHS.contains(&request.uri().path()) {
        return Ok(next.run(request).await);
    }

    if headers.get(header::AUTHORIZATION).is_some() {
        return Ok(next.run(request).await);
    }

    if let Some(origin) = headers
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok())
        && state.allowed_browser_origins.contains(origin)
    {
        return Ok(next.run(request).await);
    }

    if let Some(referer) = headers
        .get(header::REFERER)
        .and_then(|value| value.to_str().ok())
        && state.allowed_browser_origins.contains(referer)
    {
        return Ok(next.run(request).await);
    }

    Err(AppError::forbidden(
        "invalid_origin",
        "Request origin is not allowed.",
    ))
}

fn is_mutating_method(method: &Method) -> bool {
    matches!(
        method,
        &Method::POST | &Method::PUT | &Method::PATCH | &Method::DELETE
    )
}

#[cfg(test)]
mod tests {
    use crate::http::cors::same_origin;

    #[test]
    fn same_origin_rejects_prefix_matches() {
        assert!(!same_origin(
            "https://good.example.evil.com/path",
            "https://good.example"
        ));
    }

    #[test]
    fn same_origin_accepts_same_host_and_port() {
        assert!(same_origin(
            "https://good.example/path?foo=bar",
            "https://good.example"
        ));
    }

    #[test]
    fn same_origin_accepts_loopback_aliases() {
        assert!(same_origin(
            "http://127.0.0.1:3001/path",
            "http://localhost:3001"
        ));
    }
}
