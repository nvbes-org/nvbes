use axum::{Extension, Json, Router, extract::State, middleware::from_fn_with_state, routing::get};
use nvbes_core::http::error::ErrorEnvelope;

use crate::app::AppState;
use crate::http::{error::AppError, middleware::jwt::AuthContext};

use super::{rbac_db, types::DeveloperMeResponse};

#[path = "identity.domains.developer.routes.context.rs"]
pub(crate) mod context;
#[path = "identity.domains.developer.routes.health.rs"]
pub(crate) mod health;
#[path = "identity.domains.developer.routes.oauth.rs"]
pub(crate) mod oauth;
#[path = "identity.domains.developer.routes.sandbox.rs"]
pub(crate) mod sandbox;
#[path = "identity.domains.developer.routes.service_accounts.rs"]
pub(crate) mod service_accounts;
#[path = "identity.domains.developer.routes.tokens.rs"]
pub(crate) mod tokens;
#[path = "identity.domains.developer.routes.webhooks.rs"]
pub(crate) mod webhooks;

pub fn router(state: &AppState) -> Router<AppState> {
    Router::new()
        .route("/developer/me", get(me))
        .route("/developer/console/context", get(context::get_context))
        .route("/developer/console/overview", get(context::get_overview))
        .route(
            "/developer/console/oauth-clients",
            get(oauth::list_oauth_clients),
        )
        .route(
            "/developer/console/marketplace/apps",
            get(oauth::list_marketplace_apps),
        )
        .route(
            "/developer/console/marketplace/apps/{clientId}/submit",
            axum::routing::post(oauth::submit_marketplace_app),
        )
        .route(
            "/developer/console/marketplace/apps/{clientId}/review",
            axum::routing::post(oauth::review_marketplace_app),
        )
        .route(
            "/developer/console/scopes",
            get(oauth::list_scopes).post(oauth::create_scope),
        )
        .route(
            "/developer/console/scopes/{scopeKey}",
            axum::routing::put(oauth::update_scope).delete(oauth::delete_scope),
        )
        .route(
            "/developer/console/oauth-clients/{clientId}/consent-screen",
            get(oauth::get_consent_screen).put(oauth::upsert_consent_screen),
        )
        .route(
            "/developer/console/service-accounts",
            get(service_accounts::list_service_accounts),
        )
        .route(
            "/developer/console/oauth-clients/{clientId}/secrets",
            get(service_accounts::list_secret_versions),
        )
        .route(
            "/developer/console/oauth-clients/{clientId}/secrets/rotation",
            axum::routing::post(service_accounts::rotate_secret),
        )
        .route(
            "/developer/console/oauth-clients/{clientId}/secrets/{versionId}/revoke",
            axum::routing::post(service_accounts::revoke_secret_version),
        )
        .route("/developer/console/webhooks", get(webhooks::list_webhooks))
        .route(
            "/developer/console/webhooks/{endpointId}/deliveries",
            get(webhooks::list_deliveries),
        )
        .route(
            "/developer/console/webhooks/deliveries/{deliveryId}/replay",
            axum::routing::post(webhooks::replay_delivery),
        )
        .route("/developer/console/logs", get(webhooks::list_logs))
        .route(
            "/developer/console/tokens/debug",
            axum::routing::post(tokens::debug_token),
        )
        .route(
            "/developer/console/sandbox",
            get(sandbox::get_sandbox).put(sandbox::upsert_sandbox),
        )
        .route(
            "/developer/console/sandbox/reset",
            axum::routing::post(sandbox::reset_sandbox),
        )
        .route(
            "/developer/console/health-checks",
            get(health::list_health_checks).post(health::run_health_checks),
        )
        .layer(from_fn_with_state(
            state.clone(),
            crate::http::middleware::jwt::jwt_auth_middleware,
        ))
}

#[utoipa::path(
    get,
    path = "/developer/me",
    tag = "developer",
    responses(
        (status = 200, description = "Current developer portal access", body = DeveloperMeResponse),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 403, description = "Tenant context required", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub async fn me(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<DeveloperMeResponse>, AppError> {
    let tenant_id = auth.tenant_id.ok_or_else(|| {
        AppError::forbidden(
            "tenant_context_required",
            "Tenant context is required for developer portal access.",
        )
    })?;
    let roles = rbac_db::load_developer_roles(&state.db, tenant_id, auth.user_id).await?;
    let permissions = rbac_db::permissions_for_roles(&roles);

    Ok(Json(DeveloperMeResponse::new(
        tenant_id,
        roles,
        permissions,
    )))
}
