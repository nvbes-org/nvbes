use axum::{
    extract::State,
    http::{HeaderMap, StatusCode, request::Parts},
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::app::AppState;
use crate::http::error::AppError;
use crate::http::middleware::jwt::AuthContext;

pub const D_POP_HEADER: &str = "DPoP";
pub const D_POP_NONCE_HEADER: &str = "DPoP-Nonce";

#[derive(Debug, Clone)]
pub struct DpopContext {
    pub jkt: String,
}

pub async fn dpop_auth_middleware(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut request: axum::http::Request<axum::body::Body>,
    next: Next,
) -> Result<Response, AppError> {
    let auth_context = request.extensions().get::<AuthContext>().cloned();
    let token_bound_jkt = auth_context.as_ref().and_then(|ac| ac.cnf_jkt.clone());

    let dpop_header = headers
        .get(D_POP_HEADER)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    if dpop_header.is_empty() && token_bound_jkt.is_some() {
        return Err(missing_dpop_error(&state));
    }

    if dpop_header.is_empty() {
        return Ok(next.run(request).await);
    }

    let access_token = extract_access_token(&headers);
    let method = request.method().to_string();
    let uri = request.uri().to_string();
    let htu = build_htu(&headers, &uri);

    let dpop_proof =
        nvbes_dpop::verify_dpop_proof(dpop_header, &method, &htu, access_token.as_deref(), 300)
            .map_err(|e| dpop_verification_error(&e, &state))?;

    let jkt = nvbes_dpop::jwk_thumbprint(&dpop_proof.jwk);

    if let Some(ref bound_jkt) = token_bound_jkt
        && bound_jkt != &jkt
    {
        return Err(AppError::unauthorized(
            "dpop_key_mismatch",
            "DPoP key does not match the token-bound key",
        ));
    }

    if let Some(ref nonce) = dpop_proof.claims.nonce
        && let Some(ref store) = state.dpop_nonce
        && !store
            .consume(nonce)
            .await
            .map_err(|error| AppError::internal("dpop_nonce_store_error", error.to_string()))?
    {
        return Err(dpop_bad_nonce_error(&state));
    }

    let dpop_ctx = DpopContext { jkt };
    request.extensions_mut().insert(dpop_ctx);

    let mut response = next.run(request).await;

    if let Some(ref store) = state.dpop_nonce {
        let nonce = store
            .generate()
            .await
            .map_err(|error| AppError::internal("dpop_nonce_store_error", error.to_string()))?;
        response.headers_mut().insert(
            axum::http::HeaderName::from_static(D_POP_NONCE_HEADER),
            nonce.parse().unwrap(),
        );
    }

    Ok(response)
}

fn extract_access_token(headers: &HeaderMap) -> Option<String> {
    if let Some(auth) = headers.get("Authorization")
        && let Ok(auth_str) = auth.to_str()
    {
        if let Some(token) = auth_str.strip_prefix("DPoP ") {
            return Some(token.to_string());
        }
        if let Some(token) = auth_str.strip_prefix("Bearer ") {
            return Some(token.to_string());
        }
    }
    None
}

fn build_htu(headers: &HeaderMap, uri: &str) -> String {
    let scheme = headers
        .get("X-Forwarded-Proto")
        .and_then(|h| h.to_str().ok())
        .or_else(|| {
            headers
                .get("X-Forwarded-Scheme")
                .and_then(|h| h.to_str().ok())
        })
        .unwrap_or("https");

    let host = headers
        .get("X-Forwarded-Host")
        .and_then(|h| h.to_str().ok())
        .or_else(|| headers.get("Host").and_then(|h| h.to_str().ok()))
        .unwrap_or("localhost");

    let path_and_query = if uri.starts_with('/') {
        uri.to_string()
    } else {
        format!("/{}", uri)
    };

    format!("{}://{}{}", scheme, host, path_and_query)
}

fn dpop_verification_response(_state: &AppState, error: &str, description: &str) -> Response {
    let body = serde_json::json!({
        "error": error,
        "error_description": description,
    });

    let mut response = axum::Json(body).into_response();
    *response.status_mut() = StatusCode::UNAUTHORIZED;

    response
}

fn dpop_verification_error(e: &nvbes_dpop::proof::DpopError, _state: &AppState) -> AppError {
    AppError::unauthorized("dpop_invalid", e.to_string())
}

fn missing_dpop_error(state: &AppState) -> AppError {
    let _response = dpop_verification_response(
        state,
        "use_dpop_nonce",
        "DPoP proof is required for this token",
    );
    AppError::unauthorized("dpop_missing", "DPoP proof is required for this token")
}

fn dpop_bad_nonce_error(state: &AppState) -> AppError {
    let _response =
        dpop_verification_response(state, "use_dpop_nonce", "Invalid or reused DPoP nonce");
    AppError::unauthorized("dpop_bad_nonce", "Invalid or reused DPoP nonce")
}

pub trait DpopContextExtractor {
    fn dpop_context(&self) -> Option<&DpopContext>;
}

impl DpopContextExtractor for Parts {
    fn dpop_context(&self) -> Option<&DpopContext> {
        self.extensions.get::<DpopContext>()
    }
}
