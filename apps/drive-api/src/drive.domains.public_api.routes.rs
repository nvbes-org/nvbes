use axum::{
    Router,
    routing::{delete, get, patch, post},
};

use super::mgmt_handlers::*;
use super::v1_handlers::*;
use crate::app::AppState;

pub fn router(_state: &AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/workspaces/{workspaceId}/api-keys",
            get(list_api_keys).post(create_api_key),
        )
        .route(
            "/workspaces/{workspaceId}/api-keys/{apiKeyId}",
            delete(revoke_api_key),
        )
        .route("/v1/me", get(me))
        .route("/v1/workspaces", get(list_workspaces))
        .route("/v1/workspaces/{workspaceId}/objects", get(list_objects))
        .route("/v1/workspaces/{workspaceId}/folders", post(create_folder))
        .route(
            "/v1/workspaces/{workspaceId}/objects/{objectId}",
            patch(rename_object),
        )
        .route(
            "/v1/workspaces/{workspaceId}/objects/{objectId}/move",
            post(move_object),
        )
        .route(
            "/v1/workspaces/{workspaceId}/objects/{objectId}/trash",
            post(trash_object),
        )
        .route("/v1/workspaces/{workspaceId}/uploads", post(create_upload))
        .route(
            "/v1/workspaces/{workspaceId}/uploads/{uploadId}/complete",
            post(complete_upload),
        )
        .route(
            "/v1/workspaces/{workspaceId}/uploads/{uploadId}/cancel",
            post(cancel_upload),
        )
        .route(
            "/v1/workspaces/{workspaceId}/objects/{objectId}/download-url",
            post(create_download_url),
        )
        .route(
            "/v1/workspaces/{workspaceId}/share-links",
            get(list_share_links),
        )
        .route(
            "/v1/workspaces/{workspaceId}/objects/{objectId}/share-links",
            post(create_share_link),
        )
        .route(
            "/v1/workspaces/{workspaceId}/share-links/{shareLinkId}",
            patch(update_share_link).delete(revoke_share_link),
        )
        .route("/v1/workspaces/{workspaceId}/quota", get(get_quota))
        .route(
            "/v1/workspaces/{workspaceId}/audit-events",
            get(list_audit_events),
        )
        .layer(axum::middleware::from_fn(
            |req, next: axum::middleware::Next| async {
                let mut response = next.run(req).await;
                nvbes_core::http::deprecation::insert_deprecation_headers(
                    response.headers_mut(),
                    &nvbes_core::http::deprecation::SunsetDeprecation::new(
                        chrono::TimeZone::with_ymd_and_hms(&chrono::Utc, 2026, 12, 31, 23, 59, 59)
                            .single()
                            .expect("valid sunset date"),
                        true,
                    ),
                );
                response
            },
        ))
}
