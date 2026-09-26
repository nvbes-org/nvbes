use axum::{Form, Json, extract::State, http::StatusCode};
use uuid::Uuid;

use crate::app::IdentityState;
use crate::oauth_clients::{
    get_oauth_client, validate_and_consume_authorization_code, validate_client_credentials,
};
use crate::refresh::{create_refresh_token, rotate_refresh_token};
use crate::tokens::TokenService;

use super::types::{TokenErrorResponse, TokenRequest, TokenResponse, validate_scope};

pub async fn token(
    State(state): State<IdentityState>,
    Form(req): Form<TokenRequest>,
) -> Result<(StatusCode, Json<TokenResponse>), (StatusCode, Json<TokenErrorResponse>)> {
    if req.scope.as_deref().is_some_and(|s| !validate_scope(s)) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(TokenErrorResponse {
                error: "invalid_scope".to_string(),
                error_description: Some("Invalid scope format".to_string()),
            }),
        ));
    }

    match req.grant_type.as_str() {
        "authorization_code" => handle_authorization_code(&state, req).await,
        "refresh_token" => handle_refresh_token(&state, req).await,
        _ => Err((
            StatusCode::BAD_REQUEST,
            Json(TokenErrorResponse {
                error: "unsupported_grant_type".to_string(),
                error_description: Some(format!(
                    "Grant type '{}' is not supported",
                    req.grant_type
                )),
            }),
        )),
    }
}

fn issue_access_token(
    state: &IdentityState,
    principal_id: Uuid,
    session_id: Uuid,
    scope: &str,
) -> Result<String, (StatusCode, Json<TokenErrorResponse>)> {
    let token_config = crate::tokens_config::TokenConfig::from_env(&state.config.environment)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(TokenErrorResponse {
                    error: "server_error".to_string(),
                    error_description: Some(format!("Failed to create token config: {}", e)),
                }),
            )
        })?;

    let token_service = TokenService::new(token_config).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(TokenErrorResponse {
                error: "server_error".to_string(),
                error_description: Some(format!("Failed to create token service: {}", e)),
            }),
        )
    })?;

    let audience = if scope.contains("account") {
        "account"
    } else {
        "default"
    };

    token_service
        .issue(
            principal_id,
            session_id,
            audience,
            scope,
            vec!["pwd".to_string()],
        )
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(TokenErrorResponse {
                    error: "server_error".to_string(),
                    error_description: Some(format!("Failed to issue access token: {}", e)),
                }),
            )
        })
}

async fn handle_authorization_code(
    state: &IdentityState,
    req: TokenRequest,
) -> Result<(StatusCode, Json<TokenResponse>), (StatusCode, Json<TokenErrorResponse>)> {
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
            ));
        }
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(TokenErrorResponse {
                    error: "server_error".to_string(),
                    error_description: Some(format!("Database error: {}", e)),
                }),
            ));
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
                ));
            }
            Err(e) => {
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(TokenErrorResponse {
                        error: "server_error".to_string(),
                        error_description: Some(format!("Database error: {}", e)),
                    }),
                ));
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
            ));
        }
    };

    let access_token = issue_access_token(
        state,
        code_info.principal_id,
        code_info.session_id,
        &code_info.scope,
    )?;

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
            ));
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

async fn handle_refresh_token(
    state: &IdentityState,
    req: TokenRequest,
) -> Result<(StatusCode, Json<TokenResponse>), (StatusCode, Json<TokenErrorResponse>)> {
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
            (
                StatusCode::UNAUTHORIZED,
                Json(TokenErrorResponse {
                    error: "invalid_grant".to_string(),
                    error_description: Some(format!("Invalid refresh token: {}", e)),
                }),
            )
        })?;

    let access_token = issue_access_token(
        state,
        token_info.principal_id,
        token_info.session_id,
        &token_info.scope,
    )?;

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

#[cfg(all(test, feature = "database-tests"))]
#[path = "identity.oauth.token.tests.rs"]
mod database_tests;
