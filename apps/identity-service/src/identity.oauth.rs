use axum::{
    Router,
    extract::State,
    http::{HeaderMap, StatusCode, Uri},
    response::{IntoResponse, Redirect},
    routing::{get, post},
};
use uuid::Uuid;

use crate::app::IdentityState;
use crate::auth::hash_token;
use crate::oauth_clients::{
    CreateAuthorizationCodeParams, create_authorization_code, is_redirect_uri_allowed,
};
use crate::session::{authorize_return_to, session_token_from_headers};

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
    uri: Uri,
    req: axum::extract::Query<AuthorizeRequest>,
    headers: HeaderMap,
) -> Result<axum::response::Response, (StatusCode, String)> {
    if req.response_type != "code" {
        return Err((
            StatusCode::BAD_REQUEST,
            "Invalid response_type, only 'code' is supported".to_string(),
        ));
    }

    if !validate_client_id(&req.client_id) {
        return Err((
            StatusCode::BAD_REQUEST,
            "Invalid client_id format".to_string(),
        ));
    }

    if !validate_redirect_uri(&req.redirect_uri) {
        return Err((
            StatusCode::BAD_REQUEST,
            "Invalid redirect_uri format".to_string(),
        ));
    }

    if req.scope.as_deref().is_some_and(|s| !validate_scope(s)) {
        return Err((StatusCode::BAD_REQUEST, "Invalid scope format".to_string()));
    }

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

    let Some(session_token) = session_token_from_headers(&headers) else {
        return Ok(unauthenticated_authorize_response(&state, &uri));
    };

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

    let token_hash = hash_token(&session_token);
    let (principal_id, session_id) = match sqlx::query_as::<_, (Uuid, Uuid)>(
        "SELECT principal_id, id FROM identity_sessions 
             WHERE token_hash = $1 AND revoked_at IS NULL AND expires_at > clock_timestamp()",
    )
    .bind(&token_hash)
    .fetch_optional(&state.db)
    .await
    {
        Ok(Some(row)) => row,
        Ok(None) => return Ok(unauthenticated_authorize_response(&state, &uri)),
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
            ));
        }
    };

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

    let state_param = req.state.as_deref().unwrap_or("");
    Ok(Redirect::to(&format!(
        "{}?code={}&state={}",
        req.redirect_uri, code, state_param
    ))
    .into_response())
}

fn unauthenticated_authorize_response(
    state: &IdentityState,
    uri: &Uri,
) -> axum::response::Response {
    let return_to = authorize_return_to(uri);
    if state.config.login_url.is_empty() {
        return (
            StatusCode::UNAUTHORIZED,
            axum::Json(serde_json::json!({
                "error": "authentication_required",
                "return_to": return_to,
            })),
        )
            .into_response();
    }

    let separator = if state.config.login_url.contains('?') {
        '&'
    } else {
        '?'
    };
    Redirect::temporary(&format!(
        "{}{separator}return_to={}",
        state.config.login_url,
        urlencoding::encode(&return_to)
    ))
    .into_response()
}

#[cfg(test)]
#[path = "identity.oauth.tests.rs"]
mod tests;
