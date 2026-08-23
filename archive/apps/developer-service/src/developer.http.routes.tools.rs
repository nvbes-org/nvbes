use axum::{Extension, Json, extract::State};
use chrono::DateTime;
use nvbes_core::auth::token_hash;
use serde::Deserialize;

use crate::{
    access,
    app::DeveloperAppState,
    grpc::{
        health,
        pb::nvbes::developer::v1::{
            CreateSandboxRequest, RecordTokenDebugSessionRequest, ResetSandboxRequest,
        },
        sandbox, tokens,
    },
    http::{
        auth::DeveloperAuth,
        context::request_context,
        error::AppError,
        types::{
            DebugDeveloperTokenInput, DebugDeveloperTokenResponse, DeveloperHealthChecksResponse,
            DeveloperSandboxResponse, DeveloperSandboxTenantSummary, InspectDeveloperTokenRequest,
            InspectDeveloperTokenResponse, OAuthPlaygroundExchangeRequest,
            OAuthPlaygroundExchangeResponse, UpsertDeveloperSandboxInput,
        },
    },
    rbac::DeveloperPermission,
};

#[path = "developer.http.routes.tools.views.rs"]
mod views;

use views::{
    debug_claims_view, health_checks, inactive_token, invalid_identity_contract, json_string,
    sandbox_view, split_scopes,
};

#[utoipa::path(post, path = "/developer/tokens/inspect", tag = "developer", responses((status = 200, description = "Token inspection")))]
pub async fn inspect_token(
    State(state): State<DeveloperAppState>,
    Extension(auth): Extension<DeveloperAuth>,
    Json(input): Json<InspectDeveloperTokenRequest>,
) -> Result<Json<InspectDeveloperTokenResponse>, AppError> {
    access::require_permission(&state.db, &auth, DeveloperPermission::TokensInspect).await?;
    let claims = state.identity.introspect(input.token.trim(), None).await?;
    if !claims.active || claims.tenant_id != Some(auth.tenant_id) {
        return Ok(Json(inactive_token()));
    }
    Ok(Json(InspectDeveloperTokenResponse {
        active: true,
        subject: claims.sub,
        client_id: claims.client_id,
        tenant_id: claims.tenant_id.map(|id| id.to_string()),
        scopes: split_scopes(claims.scope),
        expires_at: claims
            .exp
            .and_then(|value| DateTime::from_timestamp(value, 0)),
    }))
}

#[utoipa::path(post, path = "/developer/oauth/playground/exchange", tag = "developer", responses((status = 200, description = "OAuth playground exchange")))]
pub async fn exchange_playground_code(
    State(state): State<DeveloperAppState>,
    Extension(auth): Extension<DeveloperAuth>,
    Json(input): Json<OAuthPlaygroundExchangeRequest>,
) -> Result<Json<OAuthPlaygroundExchangeResponse>, AppError> {
    access::require_permission(&state.db, &auth, DeveloperPermission::OAuthPlayground).await?;
    let response = state
        .identity
        .exchange_authorization_code(
            &input.client_id,
            &input.code,
            &input.redirect_uri,
            &input.code_verifier,
        )
        .await?;
    Ok(Json(OAuthPlaygroundExchangeResponse {
        token_type: json_string(&response, "token_type")?,
        expires_in: response
            .get("expires_in")
            .and_then(serde_json::Value::as_i64)
            .ok_or_else(|| invalid_identity_contract("expires_in"))?,
        scope: json_string(&response, "scope")?,
    }))
}

#[utoipa::path(post, path = "/developer/console/tokens/debug", tag = "developer-console", responses((status = 200, description = "Token debug result")))]
pub async fn debug_token(
    State(state): State<DeveloperAppState>,
    Extension(auth): Extension<DeveloperAuth>,
    Json(input): Json<DebugDeveloperTokenInput>,
) -> Result<Json<DebugDeveloperTokenResponse>, AppError> {
    access::require_permission(&state.db, &auth, DeveloperPermission::ConsoleTokensInspect).await?;
    let token = input.access_token.trim();
    if token.is_empty() {
        return Err(AppError::bad_request(
            "access_token_required",
            "An access token is required.",
        ));
    }

    let hash_prefix = token_hash(token).chars().take(16).collect::<String>();
    let introspected = state.identity.introspect(token, None).await?;
    let decoded = jsonwebtoken::dangerous::insecure_decode::<DebugTokenClaims>(token)
        .ok()
        .map(|data| data.claims);
    let decision = token_decision(
        introspected.active,
        decoded
            .as_ref()
            .and_then(|claims| claims.tenant_id.as_deref()),
        auth.tenant_id,
    );
    let claims = decoded.map(debug_claims_view).transpose()?;
    tokens::record_token_debug_session(
        &state.db,
        auth.user_id,
        RecordTokenDebugSessionRequest {
            context: Some(request_context(&auth)),
            tenant_id: auth.tenant_id.to_string(),
            token_hash_prefix: hash_prefix.clone(),
            active: decision.active,
            access_decision: decision.label.to_string(),
        },
    )
    .await?;

    Ok(Json(DebugDeveloperTokenResponse {
        active: decision.active,
        access_decision: decision.label.to_string(),
        claims,
        token_hash_prefix: hash_prefix,
    }))
}

#[utoipa::path(get, path = "/developer/console/sandbox", tag = "developer-console", responses((status = 200, description = "Developer sandbox")))]
pub async fn get_sandbox(
    State(state): State<DeveloperAppState>,
    Extension(auth): Extension<DeveloperAuth>,
) -> Result<Json<DeveloperSandboxResponse>, AppError> {
    access::require_permission(&state.db, &auth, DeveloperPermission::SandboxUse).await?;
    let response = sandbox::get_sandbox(&state.db, auth.tenant_id).await?;
    Ok(Json(DeveloperSandboxResponse {
        sandbox: response.sandbox.map(sandbox_view).transpose()?,
    }))
}

#[utoipa::path(put, path = "/developer/console/sandbox", tag = "developer-console", responses((status = 200, description = "Developer sandbox created")))]
pub async fn upsert_sandbox(
    State(state): State<DeveloperAppState>,
    Extension(auth): Extension<DeveloperAuth>,
    Json(input): Json<UpsertDeveloperSandboxInput>,
) -> Result<Json<DeveloperSandboxTenantSummary>, AppError> {
    access::require_permission(&state.db, &auth, DeveloperPermission::SandboxUse).await?;
    let sandbox = sandbox::create_sandbox(
        &state.db,
        CreateSandboxRequest {
            context: Some(request_context(&auth)),
            tenant_id: auth.tenant_id.to_string(),
            template: input.data_profile.unwrap_or_default(),
        },
    )
    .await?;
    Ok(Json(sandbox_view(sandbox)?))
}

#[utoipa::path(post, path = "/developer/console/sandbox/reset", tag = "developer-console", responses((status = 200, description = "Developer sandbox reset")))]
pub async fn reset_sandbox(
    State(state): State<DeveloperAppState>,
    Extension(auth): Extension<DeveloperAuth>,
) -> Result<Json<DeveloperSandboxTenantSummary>, AppError> {
    access::require_permission(&state.db, &auth, DeveloperPermission::SandboxUse).await?;
    let sandbox = sandbox::reset_sandbox(
        &state.db,
        ResetSandboxRequest {
            context: Some(request_context(&auth)),
            tenant_id: auth.tenant_id.to_string(),
            sandbox_id: String::new(),
        },
    )
    .await?;
    Ok(Json(sandbox_view(sandbox)?))
}

#[utoipa::path(get, path = "/developer/console/health-checks", tag = "developer-console", responses((status = 200, description = "Developer health checks")))]
pub async fn list_health_checks(
    State(state): State<DeveloperAppState>,
    Extension(auth): Extension<DeveloperAuth>,
) -> Result<Json<DeveloperHealthChecksResponse>, AppError> {
    access::require_permission(
        &state.db,
        &auth,
        DeveloperPermission::ConsoleHealthChecksRead,
    )
    .await?;
    let response = health::list_health_checks(&state.db, auth.tenant_id).await?;
    Ok(Json(DeveloperHealthChecksResponse {
        checks: health_checks(response.checks)?,
    }))
}

#[utoipa::path(post, path = "/developer/console/health-checks", tag = "developer-console", responses((status = 200, description = "Developer health checks executed")))]
pub async fn run_health_checks(
    State(state): State<DeveloperAppState>,
    Extension(auth): Extension<DeveloperAuth>,
) -> Result<Json<DeveloperHealthChecksResponse>, AppError> {
    access::require_permission(&state.db, &auth, DeveloperPermission::HealthChecksRun).await?;
    let response = health::run_health_checks(&state.db, auth.tenant_id).await?;
    Ok(Json(DeveloperHealthChecksResponse {
        checks: health_checks(response.checks)?,
    }))
}

#[derive(Debug, Deserialize)]
pub(super) struct DebugTokenClaims {
    sub: String,
    tenant_id: Option<String>,
    workspace_id: Option<String>,
    client_id: Option<String>,
    scope: String,
    aud: String,
    iss: String,
    exp: i64,
    iat: i64,
    nbf: i64,
    token_type: String,
    #[serde(default)]
    amr: Vec<String>,
    acr: Option<String>,
}

struct TokenDecision {
    active: bool,
    label: &'static str,
}

fn token_decision(
    active: bool,
    token_tenant_id: Option<&str>,
    expected_tenant_id: uuid::Uuid,
) -> TokenDecision {
    if token_tenant_id != Some(expected_tenant_id.to_string().as_str()) {
        TokenDecision {
            active: false,
            label: "tenant_mismatch",
        }
    } else if !active {
        TokenDecision {
            active: false,
            label: "expired",
        }
    } else {
        TokenDecision {
            active: true,
            label: "allowed",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::token_decision;
    use uuid::Uuid;

    #[test]
    fn token_decision_prioritizes_tenant_mismatch() {
        let tenant_id = Uuid::new_v4();
        let decision = token_decision(false, Some(&Uuid::new_v4().to_string()), tenant_id);
        assert_eq!(decision.label, "tenant_mismatch");
        assert!(!decision.active);
    }
}
