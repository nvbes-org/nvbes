use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    routing::{delete, get, post},
};
use nvbes_core::http::error::ErrorEnvelope;
use uuid::Uuid;

use crate::{
    app::AppState,
    domains::oauth::clients::keys::{
        OAuthClientKeyView, RotateOAuthClientKeyInput, list_keys, revoke_key, rotate_key,
    },
    http::{
        error::AppError,
        middleware::jwt::{
            AuthContext,
            account_access::{
                self, AccountAccess, OAUTH_CLIENTS_READ_SCOPE, OAUTH_CLIENTS_WRITE_SCOPE,
            },
        },
    },
};

pub(super) fn router(state: &AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/{clientId}/keys",
            account_access::protected_method(
                state,
                AccountAccess::OAuthScope(OAUTH_CLIENTS_READ_SCOPE),
                get(list_client_keys),
            ),
        )
        .route(
            "/{clientId}/keys",
            account_access::protected_method(
                state,
                AccountAccess::OAuthScope(OAUTH_CLIENTS_WRITE_SCOPE),
                post(rotate_client_key).layer(axum::middleware::from_fn(
                    crate::http::middleware::idempotency::require_idempotency_key,
                )),
            ),
        )
        .route(
            "/{clientId}/keys/{keyId}",
            account_access::protected_method(
                state,
                AccountAccess::OAuthScope(OAUTH_CLIENTS_WRITE_SCOPE),
                delete(revoke_client_key).layer(axum::middleware::from_fn(
                    crate::http::middleware::idempotency::require_idempotency_key,
                )),
            ),
        )
}

#[utoipa::path(
    get,
    path = "/oauth/clients/{clientId}/keys",
    tag = "oauth",
    params(("clientId" = String, Path)),
    responses(
        (status = 200, description = "Client verification keys", body = [OAuthClientKeyView]),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 404, description = "Client not found", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn list_client_keys(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(client_id): Path<String>,
) -> Result<Json<Vec<OAuthClientKeyView>>, AppError> {
    Ok(Json(list_keys(&state.db, &auth, &client_id).await?))
}

#[utoipa::path(
    post,
    path = "/oauth/clients/{clientId}/keys",
    tag = "oauth",
    params(("clientId" = String, Path)),
    request_body = RotateOAuthClientKeyInput,
    responses(
        (status = 200, description = "Client key rotated", body = OAuthClientKeyView),
        (status = 400, description = "Invalid key", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 404, description = "Client not found", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn rotate_client_key(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(client_id): Path<String>,
    Json(input): Json<RotateOAuthClientKeyInput>,
) -> Result<Json<OAuthClientKeyView>, AppError> {
    super::require_management_step_up(&state.redis, &auth).await?;
    Ok(Json(rotate_key(&state.db, &auth, &client_id, input).await?))
}

#[utoipa::path(
    delete,
    path = "/oauth/clients/{clientId}/keys/{keyId}",
    tag = "oauth",
    params(("clientId" = String, Path), ("keyId" = Uuid, Path)),
    responses(
        (status = 200, description = "Client key revoked", body = OAuthClientKeyView),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 404, description = "Key not found", body = ErrorEnvelope),
        (status = 409, description = "Last required key", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn revoke_client_key(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path((client_id, key_id)): Path<(String, Uuid)>,
) -> Result<Json<OAuthClientKeyView>, AppError> {
    super::require_management_step_up(&state.redis, &auth).await?;
    Ok(Json(
        revoke_key(&state.db, &auth, &client_id, key_id).await?,
    ))
}
