use axum::{Json, Router, extract::State, http::StatusCode, routing::post};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::app::IdentityState;

#[derive(Debug, Serialize)]
pub struct AuthContext {
    pub principal_id: Uuid,
    pub principal_kind: String,
    pub user_id: Option<Uuid>,
    pub session_id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub workspace_id: Option<Uuid>,
    pub client_id: String,
    pub acr: String,
    pub amr: Vec<String>,
    pub device_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AuthzDecisionRequest {
    pub token: String,
    pub resource: String,
    pub action: String,
    pub workspace_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct AuthzDecisionResponse {
    pub allowed: bool,
    pub reason: Option<String>,
    pub context: AuthContext,
}

#[derive(Debug, Deserialize)]
pub struct IntrospectRequest {
    pub token: String,
    pub token_type_hint: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct IntrospectResponse {
    pub active: bool,
    pub scope: Option<String>,
    pub client_id: Option<String>,
    pub principal_id: Option<Uuid>,
    pub exp: Option<i64>,
    pub iat: Option<i64>,
    pub sub: Option<String>,
    pub aud: Option<String>,
    pub iss: Option<String>,
}

pub fn router(state: &IdentityState) -> Router {
    Router::new()
        .route("/oauth/introspect", post(introspect))
        .route("/api/v1/authz/decision", post(authz_decision))
        .with_state(state.clone())
}

async fn introspect(
    State(state): State<IdentityState>,
    Json(req): Json<IntrospectRequest>,
) -> Result<(StatusCode, Json<IntrospectResponse>), (StatusCode, Json<serde_json::Value>)> {
    // Determine expected audience from token_type_hint or default to account
    let expected_audience = req.token_type_hint.as_deref().unwrap_or("account");

    let token_config = match crate::tokens_config::TokenConfig::from_env(&state.config.environment) {
        Ok(config) => config,
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("Failed to create token config: {}", e) })),
            ))
        }
    };

    let token_service = match crate::tokens::TokenService::new(token_config) {
        Ok(service) => service,
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("Failed to create token service: {}", e) })),
            ))
        }
    };

    match token_service.introspect(&state.db, &req.token, expected_audience).await {
        Ok(Some(claims)) => Ok((
            StatusCode::OK,
            Json(IntrospectResponse {
                active: true,
                scope: Some(claims.scope.clone()),
                client_id: None,
                principal_id: Uuid::parse_str(&claims.sub).ok(),
                exp: Some(claims.exp as i64),
                iat: Some(claims.iat as i64),
                sub: Some(claims.sub),
                aud: Some(claims.aud),
                iss: Some(claims.iss),
            }),
        )),
        Ok(None) => Ok((
            StatusCode::OK,
            Json(IntrospectResponse {
                active: false,
                scope: None,
                client_id: None,
                principal_id: None,
                exp: None,
                iat: None,
                sub: None,
                aud: None,
                iss: None,
            }),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("Introspection failed: {}", e) })),
        )),
    }
}

async fn authz_decision(
    State(state): State<IdentityState>,
    Json(req): Json<AuthzDecisionRequest>,
) -> Result<(StatusCode, Json<AuthzDecisionResponse>), (StatusCode, Json<serde_json::Value>)> {
    // Determine expected audience from request or default to account
    let expected_audience = "account";

    let token_config = match crate::tokens_config::TokenConfig::from_env(&state.config.environment) {
        Ok(config) => config,
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("Failed to create token config: {}", e) })),
            ))
        }
    };

    let token_service = match crate::tokens::TokenService::new(token_config) {
        Ok(service) => service,
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("Failed to create token service: {}", e) })),
            ))
        }
    };

    let claims = match token_service.introspect(&state.db, &req.token, expected_audience).await {
        Ok(Some(claims)) => claims,
        Ok(None) => {
            return Ok((
                StatusCode::OK,
                Json(AuthzDecisionResponse {
                    allowed: false,
                    reason: Some("Token is invalid or expired".to_string()),
                    context: AuthContext {
                        principal_id: Uuid::nil(),
                        principal_kind: "unknown".to_string(),
                        user_id: None,
                        session_id: Uuid::nil(),
                        tenant_id: None,
                        workspace_id: req.workspace_id,
                        client_id: "unknown".to_string(),
                        acr: "unknown".to_string(),
                        amr: vec![],
                        device_id: None,
                    },
                }),
            ))
        }
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("Introspection failed: {}", e) })),
            ))
        }
    };

    let principal_id = Uuid::parse_str(&claims.sub).unwrap_or(Uuid::nil());
    let session_id = Uuid::parse_str(&claims.sid).unwrap_or(Uuid::nil());

    // V1 authorization logic:
    // - Check if token has required scope for the action
    // - If workspace_id is provided, the principal must be a member (deferred to Account service)
    // - Auth level (acr/amr) is informative for V1, not enforced
    
    // For V1, allow access if token is valid and has any scope
    // Workspace membership and RBAC are deferred to Account service
    let allowed = !claims.scope.is_empty();
    let reason = if allowed {
        None
    } else {
        Some("Token has no authorized scopes".to_string())
    };

    Ok((
        StatusCode::OK,
        Json(AuthzDecisionResponse {
            allowed,
            reason,
            context: AuthContext {
                principal_id,
                principal_kind: "human".to_string(),
                user_id: Some(principal_id),
                session_id,
                tenant_id: None,
                workspace_id: req.workspace_id,
                client_id: claims.aud.clone(),
                acr: claims.amr.first().unwrap_or(&"unknown".to_string()).clone(),
                amr: claims.amr,
                device_id: None,
            },
        }),
    ))
}