use super::TokenState;
use crate::{oauth::error::OAuthError, tokens_error::TokenError};
use axum::{
    Json,
    body::to_bytes,
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};

pub(super) async fn userinfo(State(state): State<TokenState>, request: Request) -> Response {
    let result = process(&state, request).await;
    let mut response = match result {
        Ok(body) => Json(body).into_response(),
        Err((status, error, scheme)) => {
            let mut response = (status, Json(serde_json::json!({"error":error}))).into_response();
            if status == StatusCode::UNAUTHORIZED {
                response.headers_mut().insert(
                    "www-authenticate",
                    format!("{scheme} error=\"{error}\"").parse().unwrap(),
                );
            }
            response
        }
    };
    response
        .headers_mut()
        .insert("cache-control", "no-store".parse().unwrap());
    response
        .headers_mut()
        .insert("pragma", "no-cache".parse().unwrap());
    response
}

type Failure = (StatusCode, &'static str, &'static str);

async fn process(state: &TokenState, request: Request) -> Result<serde_json::Value, Failure> {
    let malformed = (StatusCode::BAD_REQUEST, "invalid_request", "Bearer");
    if request.uri().query().is_some() {
        return Err(malformed);
    }
    let (parts, body) = request.into_parts();
    if !to_bytes(body, 8192)
        .await
        .map_err(|_| malformed)?
        .is_empty()
    {
        return Err(malformed);
    }
    let authorization = header(&parts.headers, "authorization")?.ok_or((
        StatusCode::UNAUTHORIZED,
        "invalid_token",
        "Bearer",
    ))?;
    let (scheme, token) = authorization.split_once(' ').ok_or(malformed)?;
    let dpop = scheme.eq_ignore_ascii_case("dpop");
    if !dpop && !scheme.eq_ignore_ascii_case("bearer") {
        return Err(malformed);
    }
    let token = token.trim_start_matches(' ');
    if token.is_empty() || token.bytes().any(|b| b.is_ascii_whitespace()) {
        return Err(malformed);
    }
    let proof = header(&parts.headers, "dpop")?;
    state
        .tokens
        .userinfo(
            &state.db,
            &state.clients,
            token,
            dpop,
            proof,
            parts.method.as_str(),
        )
        .await
        .map_err(|error| {
            let scheme = if dpop { "DPoP" } else { "Bearer" };
            match error {
                TokenError::Database(_) | TokenError::Authorization(OAuthError::Unavailable) => (
                    StatusCode::SERVICE_UNAVAILABLE,
                    "temporarily_unavailable",
                    scheme,
                ),
                TokenError::Authorization(OAuthError::InvalidDpopProof) => {
                    (StatusCode::UNAUTHORIZED, "invalid_dpop_proof", scheme)
                }
                _ => (StatusCode::UNAUTHORIZED, "invalid_token", scheme),
            }
        })
}

fn header<'a>(headers: &'a HeaderMap, name: &str) -> Result<Option<&'a str>, Failure> {
    let mut values = headers.get_all(name).iter();
    let value = values
        .next()
        .map(|v| v.to_str())
        .transpose()
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid_request", "Bearer"))?;
    if values.next().is_some() {
        return Err((StatusCode::BAD_REQUEST, "invalid_request", "Bearer"));
    }
    Ok(value)
}
