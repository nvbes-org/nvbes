use axum::{Router, middleware::from_fn_with_state, routing::get};

use crate::app::AppState;

#[path = "identity.domains.developer.routes.context.rs"]
pub(crate) mod context;

pub fn router(state: &AppState) -> Router<AppState> {
    Router::new()
        .route("/developer/context", get(context::get_context))
        .route("/developer/overview", get(context::get_overview))
        .layer(from_fn_with_state(
            state.clone(),
            crate::http::middleware::jwt::jwt_auth_middleware,
        ))
}
