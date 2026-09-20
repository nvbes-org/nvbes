use axum::{
    Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};
use uuid::Uuid;

use crate::app::IdentityState;
use crate::auth::hash_token;
use crate::oauth_clients::{
    CreateAuthorizationCodeParams, create_authorization_code, is_redirect_uri_allowed,
};

#[path = "identity.oauth.token.rs"]
pub mod token;
#[path = "identity.oauth.types.rs"]
pub mod types;

pub use token::token;
pub use types::*;

pub fn router(state: &IdentityState) -> Router {
    Router::new()
        .route("/oauth/token", post(token))
        .route("/oauth/authorize", get(authorize))
        .with_state(state.clone())
}

async fn authorize(
    State(state): State<IdentityState>,
    req: axum::extract::Query<AuthorizeRequest>,
    headers: axum::http::HeaderMap,
) -> Result<axum::response::Redirect, (StatusCode, String)> {
    // Validate response_type
    if req.response_type != "code" {
        return Err((
            StatusCode::BAD_REQUEST,
            "Invalid response_type, only 'code' is supported".to_string(),
        ));
    }

    // Validate client_id format
    if !validate_client_id(&req.client_id) {
        return Err((
            StatusCode::BAD_REQUEST,
            "Invalid client_id format".to_string(),
        ));
    }

    // Validate redirect_uri format
    if !validate_redirect_uri(&req.redirect_uri) {
        return Err((
            StatusCode::BAD_REQUEST,
            "Invalid redirect_uri format".to_string(),
        ));
    }

    // Validate scope if provided
    if req.scope.as_deref().is_some_and(|s| !validate_scope(s)) {
        return Err((StatusCode::BAD_REQUEST, "Invalid scope format".to_string()));
    }

    // Validate PKCE parameters if provided
    if let (Some(challenge), Some(method)) = (&req.code_challenge, &req.code_challenge_method) {
        if method != "plain" && method != "S256" {
            return Err((
                StatusCode::BAD_REQUEST,
                "Invalid code_challenge_method, only 'plain' and 'S256' are supported".to_string(),
            ));
        }
        if challenge.is_empty() || challenge.len() > 128 {
            return Err((
                StatusCode::BAD_REQUEST,
                "Invalid code_challenge length".to_string(),
            ));
        }
    }

    // Check if client exists and redirect_uri is allowed
    match is_redirect_uri_allowed(&state.db, &req.client_id, &req.redirect_uri).await {
        Ok(true) => {}
        Ok(false) => {
            return Err((
                StatusCode::BAD_REQUEST,
                "Redirect URI not allowed for this client".to_string(),
            ));
        }
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
            ));
        }
    }

    // Authenticate user - check for session token in cookie or Authorization header
    let session_token = headers
        .get("cookie")
        .and_then(|h| h.to_str().ok())
        .and_then(|c| c.split('=').nth(1))
        .or_else(|| {
            headers
                .get("authorization")
                .and_then(|h| h.to_str().ok())
                .and_then(|a| a.strip_prefix("Bearer "))
        });

    let (principal_id, session_id) = if let Some(token) = session_token {
        // Validate session token and get principal_id and session_id
        let token_hash = hash_token(token);
        match sqlx::query_as::<_, (Uuid, Uuid)>(
            "SELECT principal_id, id FROM identity_sessions 
             WHERE token_hash = $1 AND revoked_at IS NULL AND expires_at > clock_timestamp()",
        )
        .bind(&token_hash)
        .fetch_optional(&state.db)
        .await
        {
            Ok(Some((pid, sid))) => (pid, sid),
            Ok(None) => {
                return Err((
                    StatusCode::UNAUTHORIZED,
                    "Invalid or expired session".to_string(),
                ));
            }
            Err(e) => {
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Database error: {}", e),
                ));
            }
        }
    } else {
        // No session token found - user must authenticate first
        return Err((
            StatusCode::UNAUTHORIZED,
            "Authentication required - no session found".to_string(),
        ));
    };

    // Create authorization code
    let scope = req.scope.clone().unwrap_or_else(|| "openid".to_string());
    let code = match create_authorization_code(
        &state.db,
        CreateAuthorizationCodeParams {
            client_id: &req.client_id,
            principal_id,
            session_id,
            redirect_uri: req.redirect_uri.clone(),
            scope,
            code_challenge: req.code_challenge.clone(),
            code_challenge_method: req.code_challenge_method.clone(),
        },
    )
    .await
    {
        Ok(c) => c,
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create authorization code: {}", e),
            ));
        }
    };

    // Redirect with authorization code
    let state_param = req.state.as_deref().unwrap_or("");
    Ok(axum::response::Redirect::to(&format!(
        "{}?code={}&state={}",
        req.redirect_uri, code, state_param
    )))
}

#[cfg(test)]
#[path = "identity.oauth.tests.rs"]
mod tests;
