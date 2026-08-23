use axum::{
    Router,
    routing::{delete, get, post, put},
};

use crate::{
    app::AppState,
    auth::{
        DELETE_SCOPE, EXPORT_SCOPE, LEGAL_READ_SCOPE, LEGAL_WRITE_SCOPE, PREFERENCES_READ_SCOPE,
        PREFERENCES_WRITE_SCOPE, PROFILE_READ_SCOPE, PROFILE_WRITE_SCOPE, SESSION_READ_SCOPE,
        SESSION_WRITE_SCOPE, protected,
    },
};

pub fn router(state: &AppState) -> Router<AppState> {
    let api = Router::new()
        .route(
            "/profile",
            protected(
                state,
                PROFILE_READ_SCOPE,
                get(crate::profile_routes::get_profile),
            ),
        )
        .route(
            "/profile",
            protected(
                state,
                PROFILE_WRITE_SCOPE,
                put(crate::profile_routes::update_profile),
            ),
        )
        .route(
            "/profile/avatar",
            protected(
                state,
                PROFILE_READ_SCOPE,
                get(crate::avatar_routes::download_avatar),
            ),
        )
        .route(
            "/profile/avatar",
            protected(
                state,
                PROFILE_WRITE_SCOPE,
                post(crate::avatar_routes::prepare_avatar_upload)
                    .delete(crate::avatar_routes::delete_avatar),
            ),
        )
        .route(
            "/preferences",
            protected(
                state,
                PREFERENCES_READ_SCOPE,
                get(crate::preferences_routes::get_preferences),
            ),
        )
        .route(
            "/preferences",
            protected(
                state,
                PREFERENCES_WRITE_SCOPE,
                put(crate::preferences_routes::update_preferences),
            ),
        )
        .route(
            "/notifications",
            protected(
                state,
                PREFERENCES_READ_SCOPE,
                get(crate::notifications_routes::get_notifications),
            ),
        )
        .route(
            "/notifications",
            protected(
                state,
                PREFERENCES_WRITE_SCOPE,
                put(crate::notifications_routes::update_notifications),
            ),
        )
        .route(
            "/consents",
            protected(
                state,
                LEGAL_READ_SCOPE,
                get(crate::consents_routes::list_consents),
            ),
        )
        .route(
            "/consents",
            protected(
                state,
                LEGAL_WRITE_SCOPE,
                post(crate::consents_routes::grant_consent)
                    .delete(crate::consents_routes::revoke_consent),
            ),
        )
        .route(
            "/privacy/gpc",
            protected(
                state,
                LEGAL_READ_SCOPE,
                get(crate::privacy_routes::get_gpc_status),
            ),
        )
        .route(
            "/privacy/exports",
            protected(
                state,
                EXPORT_SCOPE,
                post(crate::privacy_routes::request_export),
            ),
        )
        .route(
            "/privacy/exports/latest",
            protected(
                state,
                EXPORT_SCOPE,
                get(crate::privacy_routes::get_latest_export),
            ),
        )
        .route(
            "/privacy/exports/{exportId}/document",
            protected(
                state,
                EXPORT_SCOPE,
                get(crate::privacy_routes::download_export),
            ),
        )
        .route(
            "/security/sessions",
            protected(
                state,
                SESSION_READ_SCOPE,
                get(crate::sessions_routes::list_sessions),
            ),
        )
        .route(
            "/security/sessions/{sessionId}",
            protected(
                state,
                SESSION_WRITE_SCOPE,
                delete(crate::sessions_routes::revoke_session),
            ),
        )
        .route(
            "/closure",
            protected(
                state,
                DELETE_SCOPE,
                post(crate::closure_routes::request_closure)
                    .get(crate::closure_routes::get_closure),
            ),
        );

    let internal = Router::new().route(
        "/identity-registrations",
        post(crate::registration_routes::project_registration),
    );

    Router::new()
        .route("/health", get(crate::health::health))
        .route("/ready", get(crate::health::ready))
        .route("/api/openapi.json", get(crate::openapi::openapi_json))
        .nest("/api/v1", api)
        .nest("/internal/v1", internal)
        .fallback(not_found)
        .method_not_allowed_fallback(method_not_allowed)
}

async fn not_found() -> crate::error::AppError {
    crate::error::AppError::not_found("route_not_found", "The requested route does not exist.")
}

async fn method_not_allowed() -> crate::error::AppError {
    crate::error::AppError::method_not_allowed("The requested HTTP method is not supported.")
}
