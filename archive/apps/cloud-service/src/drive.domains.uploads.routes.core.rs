use axum::{
    Router, middleware,
    routing::{options, post},
};

use crate::app::AppState;

#[path = "drive.domains.uploads.routes.core.create.rs"]
mod create;
#[path = "drive.domains.uploads.routes.core.lifecycle.rs"]
mod lifecycle;
#[path = "drive.domains.uploads.routes.core.types.rs"]
mod types;

pub fn router(state: &AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/workspaces/{workspaceId}/uploads",
            post(create::create_upload),
        )
        .route(
            "/workspaces/{workspaceId}/uploads",
            options(super::tus::tus_options),
        )
        .route(
            "/workspaces/{workspaceId}/uploads/{uploadId}",
            axum::routing::head(super::tus::tus_upload_head).patch(super::tus::tus_upload_patch),
        )
        .route(
            "/workspaces/{workspaceId}/uploads/{uploadId}",
            options(super::tus::tus_options),
        )
        .route(
            "/workspaces/{workspaceId}/uploads/{uploadId}/complete",
            post(lifecycle::complete_upload),
        )
        .route(
            "/workspaces/{workspaceId}/uploads/{uploadId}/cancel",
            post(lifecycle::cancel_upload),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            crate::http::expect_continue::authenticate_before_body,
        ))
}
