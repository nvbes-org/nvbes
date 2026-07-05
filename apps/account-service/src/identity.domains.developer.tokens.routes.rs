use axum::{Extension, Json, Router, extract::State, routing::post};
use chrono::{DateTime, Utc};
use nvbes_core::http::error::ErrorEnvelope;
use uuid::Uuid;

use crate::{
    app::AppState,
    domains::{
        developer::{
            rbac::{self, DeveloperPermission},
            rbac_db,
            types::{
                InspectDeveloperTokenRequest, InspectDeveloperTokenResponse,
                OAuthPlaygroundExchangeRequest, OAuthPlaygroundExchangeResponse,
            },
        },
        oauth::service::ExchangeCodeInput,
    },
    http::{
        error::AppError,
        middleware::jwt::{AuthContext, jwt_auth_middleware},
    },
};

pub fn router(state: &AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/developer/tokens/inspect",
            post(inspect_token).layer(axum::middleware::from_fn_with_state(
                state.clone(),
                jwt_auth_middleware,
            )),
        )
        .route(
            "/developer/oauth/playground/exchange",
            post(exchange_playground_code).layer(axum::middleware::from_fn_with_state(
                state.clone(),
                jwt_auth_middleware,
            )),
        )
}

#[utoipa::path(
    post,
    path = "/developer/tokens/inspect",
    tag = "developer",
    request_body = InspectDeveloperTokenRequest,
    responses(
        (status = 200, description = "Developer token inspection result", body = InspectDeveloperTokenResponse),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 403, description = "Developer token inspection permission required", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub async fn inspect_token(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(request): Json<InspectDeveloperTokenRequest>,
) -> Result<Json<InspectDeveloperTokenResponse>, AppError> {
    let tenant_id =
        require_developer_permission(&state, &auth, DeveloperPermission::TokensInspect).await?;
    let Ok(claims) = state
        .jwt
        .validate_token(Some(&state.redis), request.token.trim(), "access")
        .await
    else {
        return Ok(Json(inactive_token_response()));
    };

    if claims.tenant_id.as_deref() != Some(&tenant_id.to_string()) {
        return Ok(Json(inactive_token_response()));
    }

    Ok(Json(InspectDeveloperTokenResponse {
        active: true,
        subject: Some(claims.sub),
        client_id: claims.client_id,
        tenant_id: claims.tenant_id,
        scopes: claims
            .scope
            .split_whitespace()
            .filter(|scope| !scope.is_empty())
            .map(str::to_string)
            .collect(),
        expires_at: DateTime::<Utc>::from_timestamp(claims.exp, 0),
    }))
}

#[utoipa::path(
    post,
    path = "/developer/oauth/playground/exchange",
    tag = "developer",
    request_body = OAuthPlaygroundExchangeRequest,
    responses(
        (status = 200, description = "OAuth playground exchange metadata", body = OAuthPlaygroundExchangeResponse),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 403, description = "Developer OAuth playground permission required", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub async fn exchange_playground_code(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(request): Json<OAuthPlaygroundExchangeRequest>,
) -> Result<Json<OAuthPlaygroundExchangeResponse>, AppError> {
    require_developer_permission(&state, &auth, DeveloperPermission::OAuthPlayground).await?;
    let token = crate::domains::oauth::flows::exchange_code(
        &state.db,
        &state.redis,
        &state.jwt,
        state.config.auth_refresh_token_ttl_hours,
        ExchangeCodeInput {
            code: request.code,
            client_id: request.client_id,
            client_secret: None,
            client_assertion_verified: false,
            redirect_uri: Some(request.redirect_uri),
            code_verifier: Some(request.code_verifier),
        },
    )
    .await?;

    Ok(Json(OAuthPlaygroundExchangeResponse {
        token_type: token.token_type,
        expires_in: token.expires_in,
        scope: token.scope,
    }))
}

async fn require_developer_permission(
    state: &AppState,
    auth: &AuthContext,
    required: DeveloperPermission,
) -> Result<Uuid, AppError> {
    let tenant_id = auth.tenant_id.ok_or_else(|| {
        AppError::forbidden(
            "tenant_context_required",
            "Tenant context is required for developer portal access.",
        )
    })?;
    let roles = rbac_db::load_developer_roles(&state.db, tenant_id, auth.user_id).await?;
    let permissions = rbac_db::permissions_for_roles(&roles);

    if !rbac::has_permission(&permissions, required) {
        return Err(AppError::forbidden(
            "developer_permission_required",
            format!("Developer permission {} is required.", required.as_scope()),
        ));
    }

    Ok(tenant_id)
}

fn inactive_token_response() -> InspectDeveloperTokenResponse {
    InspectDeveloperTokenResponse {
        active: false,
        subject: None,
        client_id: None,
        tenant_id: None,
        scopes: Vec::new(),
        expires_at: None,
    }
}
