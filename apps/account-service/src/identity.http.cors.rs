use std::collections::HashSet;
use std::sync::{Arc, RwLock};

use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, HeaderName, HeaderValue, Method, Request, Response, StatusCode, header},
    middleware::Next,
};
use sqlx::Row;

use crate::{app::AppState, http::error::AppError};

#[path = "identity.http.cors.origin.rs"]
mod origin;

pub use origin::{origin_from_redirect_uri, same_origin};

const ALLOWED_METHODS: &str = "GET, POST, PUT, PATCH, DELETE, OPTIONS";
const ALLOWED_HEADERS: &str = "content-type, authorization, accept, x-requested-with, idempotency-key, x-request-id, x-csrf-token, x-auth-user, baggage, traceparent, tracestate";
const EXPOSED_HEADERS: &str = "x-request-id, traceparent, tracestate, etag";

#[derive(Clone, Default)]
pub struct AllowedOriginRegistry {
    origins: Arc<RwLock<HashSet<String>>>,
}

impl AllowedOriginRegistry {
    pub fn contains(&self, origin: &str) -> bool {
        self.origins
            .read()
            .expect("allowed origin registry lock should not be poisoned")
            .iter()
            .any(|allowed| same_origin(origin, allowed))
    }

    pub fn replace(&self, origins: Vec<String>) {
        let mut guard = self
            .origins
            .write()
            .expect("allowed origin registry lock should not be poisoned");
        *guard = origins.into_iter().collect();
    }

    pub async fn refresh_from_db(
        &self,
        db: &sqlx::PgPool,
        config: &nvbes_core::config::AppConfig,
    ) -> Result<(), AppError> {
        let rows = sqlx::query(
            r#"
            SELECT redirect_uris, client_type::text AS client_type
            FROM oauth_clients
            WHERE revoked_at IS NULL
            "#,
        )
        .fetch_all(db)
        .await?;

        let mut origins = config_allowed_origins(config);
        for row in rows {
            let client_type: String = row.get("client_type");
            if !is_browser_client_type(&client_type) {
                continue;
            }

            let redirect_uris: Vec<String> = row.get("redirect_uris");
            origins.extend(
                redirect_uris
                    .iter()
                    .filter_map(|redirect_uri| origin_from_redirect_uri(redirect_uri)),
            );
        }

        self.replace(origin::expand_loopback_aliases(origins));
        Ok(())
    }
}

pub async fn cors_middleware(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Result<Response<Body>, AppError> {
    let origin = request
        .headers()
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);

    let is_preflight = request.method() == Method::OPTIONS
        && request
            .headers()
            .contains_key(header::ACCESS_CONTROL_REQUEST_METHOD);

    let Some(origin) = origin else {
        return Ok(next.run(request).await);
    };

    if !state.allowed_browser_origins.contains(&origin) {
        if is_preflight {
            return Err(AppError::forbidden(
                "invalid_origin",
                "Request origin is not allowed.",
            ));
        }

        return Ok(next.run(request).await);
    }

    if is_preflight {
        return Ok(preflight_response(&origin, request.headers()));
    }

    let mut response = next.run(request).await;
    apply_cors_headers(response.headers_mut(), &origin);
    Ok(response)
}

pub fn config_allowed_origins(config: &nvbes_core::config::AppConfig) -> Vec<String> {
    let mut origins = vec![config.web_base_url.clone(), config.api_base_url.clone()];

    if let Some(origin) = &config.staging_web_base_url {
        origins.push(origin.clone());
    }

    if let Some(origin) = &config.staging_api_base_url {
        origins.push(origin.clone());
    }

    origins.extend(config.additional_cors_origins.iter().cloned());
    origins
}

fn preflight_response(origin: &str, request_headers: &HeaderMap) -> Response<Body> {
    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::NO_CONTENT;

    apply_cors_headers(response.headers_mut(), origin);
    response.headers_mut().insert(
        header::ACCESS_CONTROL_ALLOW_METHODS,
        HeaderValue::from_static(ALLOWED_METHODS),
    );

    let allow_headers = request_headers
        .get(header::ACCESS_CONTROL_REQUEST_HEADERS)
        .cloned()
        .unwrap_or_else(|| HeaderValue::from_static(ALLOWED_HEADERS));
    response
        .headers_mut()
        .insert(header::ACCESS_CONTROL_ALLOW_HEADERS, allow_headers);

    response
}

fn apply_cors_headers(headers: &mut HeaderMap, origin: &str) {
    if let Ok(origin) = HeaderValue::from_str(origin) {
        headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, origin);
    }

    headers.insert(
        header::ACCESS_CONTROL_ALLOW_CREDENTIALS,
        HeaderValue::from_static("true"),
    );
    headers.insert(
        header::ACCESS_CONTROL_EXPOSE_HEADERS,
        HeaderValue::from_static(EXPOSED_HEADERS),
    );
    append_vary(headers, header::ORIGIN);
    append_vary(headers, header::ACCESS_CONTROL_REQUEST_METHOD);
    append_vary(headers, header::ACCESS_CONTROL_REQUEST_HEADERS);
}

fn append_vary(headers: &mut HeaderMap, value: HeaderName) {
    if let Some(existing) = headers
        .get(header::VARY)
        .and_then(|header| header.to_str().ok())
    {
        let needle = value.as_str();
        if existing
            .split(',')
            .map(str::trim)
            .any(|entry| entry.eq_ignore_ascii_case(needle))
        {
            return;
        }

        let updated = format!("{existing}, {needle}");
        if let Ok(updated) = HeaderValue::from_str(&updated) {
            headers.insert(header::VARY, updated);
        }
        return;
    }

    headers.insert(header::VARY, HeaderValue::from_str(value.as_str()).unwrap());
}

fn is_browser_client_type(client_type: &str) -> bool {
    matches!(
        client_type,
        "public" | "native" | "desktop" | "device" | "mobile" | "iot"
    )
}

#[cfg(test)]
#[path = "identity.http.cors.tests.rs"]
mod tests;
