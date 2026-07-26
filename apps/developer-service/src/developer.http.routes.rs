use axum::{
    Router,
    routing::{delete, get, patch, post, put},
};

use crate::app::DeveloperAppState;

#[path = "developer.http.routes.console.rs"]
pub(super) mod console;
#[path = "developer.http.routes.oauth.rs"]
pub(super) mod oauth;
#[path = "developer.http.routes.portal.rs"]
pub(super) mod portal;
#[path = "developer.http.routes.secrets.rs"]
pub(super) mod secrets;
#[path = "developer.http.routes.tools.rs"]
pub(super) mod tools;
#[path = "developer.http.routes.webhooks.rs"]
pub(super) mod webhooks;

pub fn router() -> Router<DeveloperAppState> {
    Router::new()
        .route("/developer/me", get(portal::me))
        .route(
            "/developer/apps",
            get(portal::list_apps).post(portal::create_app),
        )
        .route(
            "/developer/apps/{clientId}",
            get(portal::get_app).delete(portal::revoke_app),
        )
        .route(
            "/developer/apps/{clientId}/redirects",
            patch(portal::update_redirects),
        )
        .route(
            "/developer/webhooks",
            get(webhooks::list_portal_webhooks).post(webhooks::create_portal_webhook),
        )
        .route(
            "/developer/webhooks/{endpointId}",
            delete(webhooks::delete_portal_webhook),
        )
        .route("/developer/logs", get(portal::list_logs))
        .route("/developer/tokens/inspect", post(tools::inspect_token))
        .route(
            "/developer/oauth/playground/exchange",
            post(tools::exchange_playground_code),
        )
        .route("/developer/console/context", get(console::context))
        .route("/developer/console/overview", get(console::overview))
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
            post(oauth::submit_marketplace_app),
        )
        .route(
            "/developer/console/marketplace/apps/{clientId}/review",
            post(oauth::review_marketplace_app),
        )
        .route(
            "/developer/console/scopes",
            get(oauth::list_scopes).post(oauth::create_scope),
        )
        .route(
            "/developer/console/scopes/{scopeKey}",
            put(oauth::update_scope).delete(oauth::delete_scope),
        )
        .route(
            "/developer/console/oauth-clients/{clientId}/consent-screen",
            get(oauth::get_consent_screen).put(oauth::upsert_consent_screen),
        )
        .route(
            "/developer/console/service-accounts",
            get(secrets::list_service_accounts),
        )
        .route(
            "/developer/console/oauth-clients/{clientId}/secrets",
            get(secrets::list_secret_versions),
        )
        .route(
            "/developer/console/oauth-clients/{clientId}/secrets/rotation",
            post(secrets::rotate_secret),
        )
        .route(
            "/developer/console/oauth-clients/{clientId}/secrets/{versionId}/revoke",
            post(secrets::revoke_secret_version),
        )
        .route(
            "/developer/console/webhooks",
            get(webhooks::list_console_webhooks),
        )
        .route(
            "/developer/console/webhooks/{endpointId}/deliveries",
            get(webhooks::list_deliveries),
        )
        .route(
            "/developer/console/webhooks/deliveries/{deliveryId}/replay",
            post(webhooks::replay_delivery),
        )
        .route("/developer/console/logs", get(webhooks::list_api_logs))
        .route("/developer/console/tokens/debug", post(tools::debug_token))
        .route(
            "/developer/console/sandbox",
            get(tools::get_sandbox).put(tools::upsert_sandbox),
        )
        .route(
            "/developer/console/sandbox/reset",
            post(tools::reset_sandbox),
        )
        .route(
            "/developer/console/health-checks",
            get(tools::list_health_checks).post(tools::run_health_checks),
        )
}
