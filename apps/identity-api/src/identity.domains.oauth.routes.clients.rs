use axum::{
    Extension, Json, Router,
    extract::{Path, Query, State},
    routing::{delete, get, patch},
};
use nvbes_core::http::error::ErrorEnvelope;
use serde::Deserialize;
use uuid::Uuid;

use crate::domains::auth::verification;
use crate::{app::AppState, http::error::AppError, http::middleware::jwt::AuthContext};

pub fn router(state: &AppState) -> Router<AppState> {
    let auth_middleware = crate::http::middleware::jwt::jwt_auth_middleware;

    Router::new()
        .route(
            "/",
            get(list_clients)
                .post(create_client)
                .layer(axum::middleware::from_fn_with_state(
                    state.clone(),
                    auth_middleware.clone(),
                )),
        )
        .route(
            "/{clientId}/policies",
            get(list_client_policies).post(create_client_policy).layer(
                axum::middleware::from_fn_with_state(state.clone(), auth_middleware.clone()),
            ),
        )
        .route(
            "/{clientId}",
            delete(revoke_client).layer(axum::middleware::from_fn_with_state(
                state.clone(),
                auth_middleware,
            )),
        )
}

// Separate router for /client-policies to preserve original contract
pub fn policies_router(state: &AppState) -> Router<AppState> {
    let auth_middleware = crate::http::middleware::jwt::jwt_auth_middleware;

    Router::new().route(
        "/{policyId}",
        patch(update_client_policy)
            .delete(delete_client_policy)
            .layer(axum::middleware::from_fn_with_state(
                state.clone(),
                auth_middleware,
            )),
    )
}

async fn require_management_step_up(
    redis: &nvbes_redis::RedisPool,
    auth: &AuthContext,
) -> Result<(), AppError> {
    verification::require_recent_step_up(redis, auth, None).await
}

fn json_value<T: serde::Serialize>(value: T) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(serde_json::to_value(value)?))
}

#[utoipa::path(
    get,
    path = "/oauth/clients",
    tag = "oauth",
    responses(
        (status = 200, description = "List OAuth clients", body = crate::domains::oauth::service::OAuthClientsResult),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn list_clients(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = crate::domains::oauth::clients::list_clients(&state.db, &auth).await?;
    json_value(result)
}

#[utoipa::path(
    post,
    path = "/oauth/clients",
    tag = "oauth",
    request_body = crate::domains::oauth::service::CreateOAuthClientInput,
    responses(
        (status = 200, description = "OAuth client created", body = crate::domains::oauth::service::CreateOAuthClientResult),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn create_client(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(request): Json<crate::domains::oauth::service::CreateOAuthClientInput>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_management_step_up(&state.redis, &auth).await?;
    let result = crate::domains::oauth::clients::create_client(&state.db, &auth, request).await?;
    json_value(result)
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct ListPoliciesRequest {
    pub client_id: Option<String>,
}

#[utoipa::path(
    get,
    path = "/oauth/clients/{clientId}/policies",
    tag = "oauth",
    params(
        ("clientId" = String, Path, description = "OAuth client ID"),
    ),
    responses(
        (status = 200, description = "List client policies", body = crate::domains::oauth::service::OAuthClientPoliciesResult),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn list_client_policies(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Query(request): Query<ListPoliciesRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result =
        crate::domains::oauth::policies::list_client_policies(&state.db, &auth, request.client_id)
            .await?;
    json_value(result)
}

#[utoipa::path(
    post,
    path = "/oauth/clients/{clientId}/policies",
    tag = "oauth",
    params(
        ("clientId" = String, Path, description = "OAuth client ID"),
    ),
    request_body = crate::domains::oauth::service::CreateOAuthClientPolicyInput,
    responses(
        (status = 200, description = "Client policy created", body = crate::domains::oauth::service::OAuthClientPolicyView),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn create_client_policy(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(client_id): Path<String>,
    Json(request): Json<crate::domains::oauth::service::CreateOAuthClientPolicyInput>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_management_step_up(&state.redis, &auth).await?;
    let result =
        crate::domains::oauth::policies::create_client_policy(&state.db, &auth, client_id, request)
            .await?;
    json_value(result)
}

#[utoipa::path(
    patch,
    path = "/oauth/client-policies/{policyId}",
    tag = "oauth",
    params(
        ("policyId" = Uuid, Path, description = "Policy ID"),
    ),
    request_body = crate::domains::oauth::service::UpdateOAuthClientPolicyInput,
    responses(
        (status = 200, description = "Client policy updated", body = crate::domains::oauth::service::OAuthClientPolicyView),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 404, description = "Not found", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn update_client_policy(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(policy_id): Path<Uuid>,
    Json(request): Json<crate::domains::oauth::service::UpdateOAuthClientPolicyInput>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_management_step_up(&state.redis, &auth).await?;
    let result =
        crate::domains::oauth::policies::update_client_policy(&state.db, &auth, policy_id, request)
            .await?;
    json_value(result)
}

#[utoipa::path(
    delete,
    path = "/oauth/client-policies/{policyId}",
    tag = "oauth",
    params(
        ("policyId" = Uuid, Path, description = "Policy ID"),
    ),
    responses(
        (status = 200, description = "Client policy deleted", body = crate::domains::oauth::service::DeleteOAuthClientPolicyResult),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 404, description = "Not found", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn delete_client_policy(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(policy_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_management_step_up(&state.redis, &auth).await?;
    let result =
        crate::domains::oauth::policies::delete_client_policy(&state.db, &auth, policy_id).await?;
    json_value(result)
}

#[utoipa::path(
    delete,
    path = "/oauth/clients/{clientId}",
    tag = "oauth",
    params(
        ("clientId" = String, Path, description = "OAuth client ID"),
    ),
    responses(
        (status = 200, description = "OAuth client revoked", body = crate::domains::oauth::service::RevokeOAuthClientResult),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 404, description = "Not found", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn revoke_client(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(client_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_management_step_up(&state.redis, &auth).await?;
    let result =
        crate::domains::oauth::clients::revoke_client(&state.db, &state.redis, &auth, &client_id)
            .await?;
    json_value(result)
}
