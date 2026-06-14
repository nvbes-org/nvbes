use axum::{Router, middleware::from_fn_with_state, routing::get};

use crate::app::AppState;

#[path = "identity.domains.developer.routes.context.rs"]
pub(crate) mod context;
#[path = "identity.domains.developer.routes.oauth.rs"]
pub(crate) mod oauth;

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
        .layer(from_fn_with_state(
            state.clone(),
            crate::http::middleware::jwt::jwt_auth_middleware,
        ))
}
