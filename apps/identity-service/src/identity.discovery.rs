use axum::{Json, Router, extract::State, http::StatusCode, routing::get};
use serde::Serialize;

use crate::app::IdentityState;

#[derive(Debug, Serialize)]
pub struct OAuthAuthorizationServerMetadata {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub jwks_uri: String,
    pub response_types_supported: Vec<String>,
    pub grant_types_supported: Vec<String>,
    pub token_endpoint_auth_methods_supported: Vec<String>,
    pub scopes_supported: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct OpenIdConfiguration {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub jwks_uri: String,
    pub response_types_supported: Vec<String>,
    pub grant_types_supported: Vec<String>,
    pub token_endpoint_auth_methods_supported: Vec<String>,
    pub scopes_supported: Vec<String>,
    pub subject_types_supported: Vec<String>,
    pub id_token_signing_alg_values_supported: Vec<String>,
}

pub fn router(state: &IdentityState) -> Router {
    Router::new()
        .route("/.well-known/oauth-authorization-server", get(oauth_metadata))
        .route("/.well-known/openid-configuration", get(oidc_metadata))
        .route("/.well-known/jwks.json", get(jwks))
        .with_state(state.clone())
}

async fn oauth_metadata(
    State(state): State<IdentityState>,
) -> Result<Json<OAuthAuthorizationServerMetadata>, (StatusCode, String)> {
    let issuer = state.config.token_issuer.clone();

    Ok(Json(OAuthAuthorizationServerMetadata {
        issuer: issuer.clone(),
        authorization_endpoint: format!("{}/oauth/authorize", issuer),
        token_endpoint: format!("{}/oauth/token", issuer),
        jwks_uri: format!("{}/.well-known/jwks.json", issuer),
        response_types_supported: vec!["code".to_string()],
        grant_types_supported: vec!["authorization_code".to_string(), "refresh_token".to_string()],
        token_endpoint_auth_methods_supported: vec!["client_secret_post".to_string()],
        scopes_supported: vec!["openid".to_string(), "profile".to_string(), "email".to_string()],
    }))
}

async fn oidc_metadata(
    State(state): State<IdentityState>,
) -> Result<Json<OpenIdConfiguration>, (StatusCode, String)> {
    let issuer = state.config.token_issuer.clone();

    Ok(Json(OpenIdConfiguration {
        issuer: issuer.clone(),
        authorization_endpoint: format!("{}/oauth/authorize", issuer),
        token_endpoint: format!("{}/oauth/token", issuer),
        jwks_uri: format!("{}/.well-known/jwks.json", issuer),
        response_types_supported: vec!["code".to_string()],
        grant_types_supported: vec!["authorization_code".to_string(), "refresh_token".to_string()],
        token_endpoint_auth_methods_supported: vec!["client_secret_post".to_string()],
        scopes_supported: vec!["openid".to_string(), "profile".to_string(), "email".to_string()],
        subject_types_supported: vec!["public".to_string()],
        id_token_signing_alg_values_supported: vec!["RS256".to_string()],
    }))
}

async fn jwks(
    State(state): State<IdentityState>,
) -> Result<Json<crate::tokens::JsonWebKeySet>, (StatusCode, String)> {
    let token_config = match crate::tokens_config::TokenConfig::from_env(&state.config.environment) {
        Ok(config) => config,
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create token config: {}", e),
            ))
        }
    };

    let token_service = match crate::tokens::TokenService::new(token_config) {
        Ok(service) => service,
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create token service: {}", e),
            ))
        }
    };

    Ok(Json(token_service.jwks().clone()))
}