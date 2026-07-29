use axum::{
    Extension, Json, Router,
    extract::{Path, Query, State},
    routing::{delete, get, patch},
};
use nvbes_core::http::error::ErrorEnvelope;
use serde::Deserialize;
use uuid::Uuid;

use crate::domains::auth::verification;
use crate::{
    app::AppState,
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

#[path = "identity.domains.oauth.routes.clients.keys.rs"]
pub(crate) mod keys;

pub fn router(state: &AppState) -> Router<AppState> {
    Router::new()
        .merge(keys::router(state))
        .route(
            "/",
            account_access::protected_method(
                state,
                AccountAccess::OAuthScope(OAUTH_CLIENTS_READ_SCOPE),
                get(list_clients),
            ),
        )
        .route(
            "/",
            account_access::protected_method(
                state,
                AccountAccess::OAuthScope(OAUTH_CLIENTS_WRITE_SCOPE),
                axum::routing::post(create_client).layer(axum::middleware::from_fn(
                    crate::http::middleware::idempotency::require_idempotency_key,
                )),
            ),
        )
        .route(
            "/{clientId}/policies",
            account_access::protected_method(
                state,
                AccountAccess::OAuthScope(OAUTH_CLIENTS_READ_SCOPE),
                get(list_client_policies),
            ),
        )
        .route(
            "/{clientId}/policies",
            account_access::protected_method(
                state,
                AccountAccess::OAuthScope(OAUTH_CLIENTS_WRITE_SCOPE),
                axum::routing::post(create_client_policy).layer(axum::middleware::from_fn(
                    crate::http::middleware::idempotency::require_idempotency_key,
                )),
            ),
        )
        .route(
            "/{clientId}",
            account_access::protected_method(
                state,
                AccountAccess::OAuthScope(OAUTH_CLIENTS_WRITE_SCOPE),
                delete(revoke_client).layer(axum::middleware::from_fn(
                    crate::http::middleware::idempotency::require_idempotency_key,
                )),
            ),
        )
}

// Separate router for /client-policies to preserve original contract
pub fn policies_router(state: &AppState) -> Router<AppState> {
    Router::new().route(
        "/{policyId}",
        account_access::protected_method(
            state,
            AccountAccess::OAuthScope(OAUTH_CLIENTS_WRITE_SCOPE),
            patch(update_client_policy)
                .delete(delete_client_policy)
                .layer(axum::middleware::from_fn(
                    crate::http::middleware::idempotency::require_idempotency_key,
                )),
        ),
    )
}

async fn require_management_step_up(
    redis: &nvbes_redis::RedisPool,
    auth: &AuthContext,
) -> Result<(), AppError> {
    verification::require_recent_phishing_resistant_step_up(redis, auth).await
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
    Query(query): Query<ListClientsQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result =
        crate::domains::oauth::clients::list_clients(&state.db, &auth, query.limit, query.cursor)
            .await?;
    json_value(result)
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ListClientsQuery {
    pub limit: Option<i64>,
    pub cursor: Option<String>,
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
    validate_runtime_security_profile(&state, &request)?;
    let result = crate::domains::oauth::clients::create_client(&state.db, &auth, request).await?;
    state
        .allowed_browser_origins
        .refresh_from_db(&state.db, &state.config)
        .await?;
    json_value(result)
}

fn validate_runtime_security_profile(
    state: &AppState,
    request: &crate::domains::oauth::service::CreateOAuthClientInput,
) -> Result<(), AppError> {
    if request.security_profile.as_deref() != Some("high_assurance") {
        return Ok(());
    }
    if !state.config.fapi_high_assurance_enabled {
        return Err(AppError::bad_request(
            "fapi_profile_not_activated",
            "High-assurance OAuth clients are disabled until the FAPI conformance gate is enabled.",
        ));
    }
    match request.sender_constraint.as_deref() {
        Some("dpop") if state.dpop_nonce.is_none() => Err(AppError::bad_request(
            "dpop_not_available",
            "High-assurance DPoP clients require NVBES_DPOP_ENABLED.",
        )),
        Some("mtls") if !state.config.mtls_enabled => Err(AppError::bad_request(
            "mtls_not_available",
            "High-assurance mTLS clients require NVBES_MTLS_ENABLED.",
        )),
        _ => Ok(()),
    }
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ListPoliciesRequest {
    pub client_id: Option<String>,
    pub limit: Option<i64>,
    pub cursor: Option<String>,
}

#[utoipa::path(
    get,
    path = "/oauth/clients/{clientId}/policies",
    tag = "oauth",
    params(
        ("clientId" = String, Path, description = "OAuth client ID"),
        ("limit" = Option<i64>, Query, description = "Max results"),
        ("cursor" = Option<String>, Query, description = "Opaque pagination cursor"),
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
    let result = crate::domains::oauth::policies::list_client_policies(
        &state.db,
        &auth,
        request.client_id,
        request.limit,
        request.cursor,
    )
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
    state
        .allowed_browser_origins
        .refresh_from_db(&state.db, &state.config)
        .await?;
    json_value(result)
}
