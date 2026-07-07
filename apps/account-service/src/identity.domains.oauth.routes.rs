use axum::{Router, http::HeaderMap};
use base64::Engine;

use crate::{app::AppState, http::error::AppError};

#[path = "identity.domains.oauth.routes.authorize.rs"]
pub mod authorize;
#[path = "identity.domains.oauth.routes.clients.rs"]
pub mod clients;
#[path = "identity.domains.oauth.routes.device.rs"]
pub mod device;
#[path = "identity.domains.oauth.routes.introspect.rs"]
pub mod introspect;
#[path = "identity.domains.oauth.routes.par.rs"]
pub mod par;
#[path = "identity.domains.oauth.routes.revoke.rs"]
pub mod revoke;
#[path = "identity.domains.oauth.routes.token.rs"]
pub mod token;
#[path = "identity.domains.oauth.routes.userinfo.rs"]
pub mod userinfo;

pub fn router(state: &AppState) -> Router<AppState> {
    Router::new()
        .merge(authorize::router())
        .merge(par::router())
        .merge(token::router())
        .merge(userinfo::router())
        .merge(introspect::router())
        .merge(revoke::router())
        .merge(crate::domains::oauth::hosted_routes::router())
        .nest("/clients", clients::router(state))
        .nest("/device", device::router(state))
        .nest("/client-policies", clients::policies_router(state))
        .layer(axum::middleware::from_fn(
            nvbes_core::security::no_cache_headers,
        ))
}

pub fn parse_basic_client_auth(headers: &HeaderMap) -> Result<(String, String), AppError> {
    let header = headers
        .get(axum::http::header::AUTHORIZATION)
        .ok_or_else(|| {
            AppError::unauthorized(
                "invalid_client",
                "Client authentication is required for introspection.",
            )
        })?
        .to_str()
        .map_err(|_| {
            AppError::unauthorized("invalid_client", "Client authentication header is invalid.")
        })?;

    let encoded = header.strip_prefix("Basic ").ok_or_else(|| {
        AppError::unauthorized(
            "invalid_client",
            "Client authentication must use HTTP Basic auth.",
        )
    })?;

    let decoded = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|_| {
            AppError::unauthorized(
                "invalid_client",
                "Client authentication credentials are invalid.",
            )
        })?;

    let decoded = String::from_utf8(decoded).map_err(|_| {
        AppError::unauthorized(
            "invalid_client",
            "Client authentication credentials are invalid.",
        )
    })?;

    let mut parts = decoded.splitn(2, ':');
    let client_id = parts.next().unwrap_or_default().trim();
    let client_secret = parts.next().unwrap_or_default().trim();

    if client_id.is_empty() || client_secret.is_empty() {
        return Err(AppError::unauthorized(
            "invalid_client",
            "Client authentication credentials are invalid.",
        ));
    }

    Ok((client_id.to_string(), client_secret.to_string()))
}

pub fn optional_basic_client_auth(
    headers: &HeaderMap,
) -> Result<Option<(String, String)>, AppError> {
    let Some(header) = headers.get(axum::http::header::AUTHORIZATION) else {
        return Ok(None);
    };

    let header = header.to_str().map_err(|_| {
        AppError::unauthorized("invalid_client", "Client authentication header is invalid.")
    })?;

    if !header.starts_with("Basic ") {
        return Err(AppError::unauthorized(
            "invalid_client",
            "Client authentication must use HTTP Basic auth.",
        ));
    }

    parse_basic_client_auth(headers).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::header::AUTHORIZATION;
    use axum::http::{HeaderMap, HeaderValue};
    use base64::Engine;

    #[test]
    fn parse_basic_client_auth_extracts_client_credentials() {
        let mut headers = HeaderMap::new();
        let encoded = base64::engine::general_purpose::STANDARD.encode("client-a:secret-a");
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Basic {encoded}")).unwrap(),
        );

        let (client_id, client_secret) =
            parse_basic_client_auth(&headers).expect("basic auth should parse");

        assert_eq!(client_id, "client-a");
        assert_eq!(client_secret, "secret-a");
    }

    #[test]
    fn parse_basic_client_auth_rejects_missing_secret() {
        let mut headers = HeaderMap::new();
        let encoded = base64::engine::general_purpose::STANDARD.encode("client-a:");
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Basic {encoded}")).unwrap(),
        );

        let error = parse_basic_client_auth(&headers).expect_err("missing secret should fail");

        assert_eq!(error.code, "invalid_client");
    }
}
