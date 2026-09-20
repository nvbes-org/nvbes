use axum::{Json, Router, extract::State, http::StatusCode, routing::{get, post}};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::app::IdentityState;
use crate::auth::hash_token;
use crate::refresh::{rotate_refresh_token, create_refresh_token};
use crate::oauth_clients::{
    create_authorization_code,
    get_oauth_client,
    is_redirect_uri_allowed,
    validate_and_consume_authorization_code,
    validate_client_credentials,
};
use crate::tokens::TokenService;

fn validate_client_id(client_id: &str) -> bool {
    // Basic validation: client_id should be alphanumeric with hyphens/underscores
    !client_id.is_empty()
        && client_id.len() <= 128
        && client_id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
}

fn validate_redirect_uri(uri: &str) -> bool {
    // Basic validation: should be a valid URL with https (or http for localhost in dev)
    if uri.is_empty() || uri.len() > 2048 {
        return false;
    }
    // In development, allow http; in production, require https
    // For now, just check it starts with http:// or https://
    uri.starts_with("http://") || uri.starts_with("https://")
}

fn validate_scope(scope: &str) -> bool {
    // Validate scope format: space-separated tokens
    if scope.is_empty() {
        return true; // Empty scope is valid
    }
    scope
        .split_whitespace()
        .all(|token| {
            !token.is_empty()
                && token.len() <= 64
                && token
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == ':')
        })
}

#[derive(Debug, Deserialize)]
pub struct TokenRequest {
    pub grant_type: String,
    pub code: Option<String>,
    pub redirect_uri: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
    pub code_verifier: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub scope: String,
    pub refresh_token: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TokenErrorResponse {
    pub error: String,
    pub error_description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AuthorizeRequest {
    pub response_type: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub scope: Option<String>,
    pub state: Option<String>,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
}

pub fn router(state: &IdentityState) -> Router {
    Router::new()
        .route("/oauth/token", post(token))
        .route("/oauth/authorize", get(authorize))
        .with_state(state.clone())
}

async fn token(
    State(state): State<IdentityState>,
    Json(req): Json<TokenRequest>,
) -> Result<(StatusCode, Json<TokenResponse>), (StatusCode, Json<TokenErrorResponse>)> {
    match req.grant_type.as_str() {
        "authorization_code" => {
            let Some(code) = req.code else {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(TokenErrorResponse {
                        error: "invalid_request".to_string(),
                        error_description: Some("Missing authorization code".to_string()),
                    }),
                ));
            };

            let Some(client_id) = req.client_id else {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(TokenErrorResponse {
                        error: "invalid_client".to_string(),
                        error_description: Some("Missing client_id".to_string()),
                    }),
                ));
            };

            let Some(redirect_uri) = req.redirect_uri else {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(TokenErrorResponse {
                        error: "invalid_request".to_string(),
                        error_description: Some("Missing redirect_uri".to_string()),
                    }),
                ));
            };

            // Validate client credentials if confidential
            let client = match get_oauth_client(&state.db, &client_id).await {
                Ok(Some(c)) => c,
                Ok(None) => {
                    return Err((
                        StatusCode::UNAUTHORIZED,
                        Json(TokenErrorResponse {
                            error: "invalid_client".to_string(),
                            error_description: Some("Client not found".to_string()),
                        }),
                    ))
                }
                Err(e) => {
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(TokenErrorResponse {
                            error: "server_error".to_string(),
                            error_description: Some(format!("Database error: {}", e)),
                        }),
                    ))
                }
            };

            if client.is_confidential {
                let Some(client_secret) = req.client_secret else {
                    return Err((
                        StatusCode::UNAUTHORIZED,
                        Json(TokenErrorResponse {
                            error: "invalid_client".to_string(),
                            error_description: Some("Missing client_secret".to_string()),
                        }),
                    ));
                };

                match validate_client_credentials(&state.db, &client_id, &client_secret).await {
                    Ok(true) => {}
                    Ok(false) => {
                        return Err((
                            StatusCode::UNAUTHORIZED,
                            Json(TokenErrorResponse {
                                error: "invalid_client".to_string(),
                                error_description: Some("Invalid client credentials".to_string()),
                            }),
                        ))
                    }
                    Err(e) => {
                        return Err((
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(TokenErrorResponse {
                                error: "server_error".to_string(),
                                error_description: Some(format!("Database error: {}", e)),
                            }),
                        ))
                    }
                }
            }

            // Validate and consume authorization code
            let code_info = match validate_and_consume_authorization_code(
                &state.db,
                &code,
                &client_id,
                &redirect_uri,
                req.code_verifier.as_deref(),
            )
            .await
            {
                Ok(info) => info,
                Err(e) => {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        Json(TokenErrorResponse {
                            error: "invalid_grant".to_string(),
                            error_description: Some(format!("Invalid authorization code: {}", e)),
                        }),
                    ))
                }
            };

            // Issue real access token
            let token_config = match crate::tokens_config::TokenConfig::from_env(&state.config.environment) {
                Ok(config) => config,
                Err(e) => {
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(TokenErrorResponse {
                            error: "server_error".to_string(),
                            error_description: Some(format!("Failed to create token config: {}", e)),
                        }),
                    ))
                }
            };

            let token_service = match TokenService::new(token_config) {
                Ok(service) => service,
                Err(e) => {
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(TokenErrorResponse {
                            error: "server_error".to_string(),
                            error_description: Some(format!("Failed to create token service: {}", e)),
                        }),
                    ))
                }
            };

            // Determine audience from scope or use default
            let audience = if code_info.scope.contains("account") {
                "account"
            } else {
                "default"
            };

            let access_token = match token_service.issue(
                code_info.principal_id,
                code_info.session_id,
                audience,
                &code_info.scope,
                vec!["pwd".to_string()], // Get actual amr from session
            ) {
                Ok(token) => token,
                Err(e) => {
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(TokenErrorResponse {
                            error: "server_error".to_string(),
                            error_description: Some(format!("Failed to issue access token: {}", e)),
                        }),
                    ))
                }
            };

            // Issue refresh token for authorization code flow
            let refresh_token = match create_refresh_token(
                &state.db,
                code_info.principal_id,
                code_info.session_id,
                &client_id,
                &code_info.scope,
            )
            .await
            {
                Ok(info) => info.token,
                Err(e) => {
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(TokenErrorResponse {
                            error: "server_error".to_string(),
                            error_description: Some(format!("Failed to issue refresh token: {}", e)),
                        }),
                    ))
                }
            };

            Ok((
                StatusCode::OK,
                Json(TokenResponse {
                    access_token,
                    token_type: "Bearer".to_string(),
                    expires_in: 900,
                    scope: code_info.scope,
                    refresh_token: Some(refresh_token),
                }),
            ))
        }
        "refresh_token" => {
            let Some(refresh_token) = req.refresh_token else {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(TokenErrorResponse {
                        error: "invalid_request".to_string(),
                        error_description: Some("Missing refresh token".to_string()),
                    }),
                ));
            };

            let client_id = req.client_id.as_deref().unwrap_or("default_client");

            // Validate and rotate refresh token
            let token_info = rotate_refresh_token(&state.db, &refresh_token, client_id)
                .await
                .map_err(|e| {
                    (StatusCode::UNAUTHORIZED, Json(TokenErrorResponse {
                        error: "invalid_grant".to_string(),
                        error_description: Some(format!("Invalid refresh token: {}", e)),
                    }))
                })?;

            // Issue real access token
            let token_config = match crate::tokens_config::TokenConfig::from_env(&state.config.environment) {
                Ok(config) => config,
                Err(e) => {
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(TokenErrorResponse {
                            error: "server_error".to_string(),
                            error_description: Some(format!("Failed to create token config: {}", e)),
                        }),
                    ))
                }
            };

            let token_service = match TokenService::new(token_config) {
                Ok(service) => service,
                Err(e) => {
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(TokenErrorResponse {
                            error: "server_error".to_string(),
                            error_description: Some(format!("Failed to create token service: {}", e)),
                        }),
                    ))
                }
            };

            // Determine audience from scope or use default
            let audience = if token_info.scope.contains("account") {
                "account"
            } else {
                "default"
            };

            let access_token = match token_service.issue(
                token_info.principal_id,
                token_info.session_id,
                audience,
                &token_info.scope,
                vec!["pwd".to_string()], // Get actual amr from session
            ) {
                Ok(token) => token,
                Err(e) => {
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(TokenErrorResponse {
                            error: "server_error".to_string(),
                            error_description: Some(format!("Failed to issue access token: {}", e)),
                        }),
                    ))
                }
            };

            Ok((
                StatusCode::OK,
                Json(TokenResponse {
                    access_token,
                    token_type: "Bearer".to_string(),
                    expires_in: 900,
                    scope: token_info.scope,
                    refresh_token: Some(token_info.token),
                }),
            ))
        }
        _ => Err((
            StatusCode::BAD_REQUEST,
            Json(TokenErrorResponse {
                error: "unsupported_grant_type".to_string(),
                error_description: Some(format!("Grant type '{}' is not supported", req.grant_type)),
            }),
        )),
    }
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
    if let Some(scope) = &req.scope {
        if !validate_scope(scope) {
            return Err((
                StatusCode::BAD_REQUEST,
                "Invalid scope format".to_string(),
            ));
        }
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
            ))
        }
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
            ))
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
                ))
            }
            Err(e) => {
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Database error: {}", e),
                ))
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
        &req.client_id,
        principal_id,
        session_id,
        req.redirect_uri.clone(),
        scope,
        req.code_challenge.clone(),
        req.code_challenge_method.clone(),
    )
    .await
    {
        Ok(c) => c,
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create authorization code: {}", e),
            ))
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
mod oauth_tests;