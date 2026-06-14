use axum::{Router, middleware::from_fn_with_state, routing::get};

use crate::app::AppState;

#[path = "identity.domains.developer.routes.context.rs"]
pub(crate) mod context;
#[path = "identity.domains.developer.routes.oauth.rs"]
pub(crate) mod oauth;
#[path = "identity.domains.developer.routes.service_accounts.rs"]
pub(crate) mod service_accounts;
#[path = "identity.domains.developer.routes.webhooks.rs"]
pub(crate) mod webhooks;

pub fn router(state: &AppState) -> Router<AppState> {
    Router::new()
        .route("/developer/context", get(context::get_context))
        .route("/developer/overview", get(context::get_overview))
        .route("/developer/oauth-clients", get(oauth::list_oauth_clients))
        .route(
            "/developer/marketplace/apps",
            get(oauth::list_marketplace_apps),
        )
        .route("/developer/scopes", get(oauth::list_scopes))
        .route(
            "/developer/oauth-clients/{clientId}/consent-screen",
            get(oauth::get_consent_screen).put(oauth::upsert_consent_screen),
        )
        .route(
            "/developer/service-accounts",
            get(service_accounts::list_service_accounts),
        )
        .route(
            "/developer/oauth-clients/{clientId}/secrets",
            get(service_accounts::list_secret_versions),
        )
        .route(
            "/developer/oauth-clients/{clientId}/secrets/rotation",
            axum::routing::post(service_accounts::rotate_secret),
        )
        .route(
            "/developer/oauth-clients/{clientId}/secrets/{versionId}/revoke",
            axum::routing::post(service_accounts::revoke_secret_version),
        )
        .route("/developer/webhooks", get(webhooks::list_webhooks))
        .route(
            "/developer/webhooks/{endpointId}/deliveries",
            get(webhooks::list_deliveries),
        )
        .route(
            "/developer/webhooks/deliveries/{deliveryId}/replay",
            axum::routing::post(webhooks::replay_delivery),
        )
        .route("/developer/logs", get(webhooks::list_logs))
        .layer(from_fn_with_state(
            state.clone(),
            crate::http::middleware::jwt::jwt_auth_middleware,
        ))
}
