use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::{delete, get, post},
};
use nvbes_core::http::error::ErrorEnvelope;
use uuid::Uuid;

use crate::{
    app::AppState,
    domains::{
        authz::{ResourceContext, WorkspaceAction, authorize_workspace_action},
        service_accounts::types::{
            AttachOAuthClientInput, CreateServiceAccountInput,
            CreateServiceAccountOAuthClientInput, CreateServiceAccountOAuthClientResult,
            RotateOAuthClientSecretResult, ServiceAccountView, ServiceAccountsResult,
            UpdateServiceAccountInput,
        },
    },
    http::{
        error::AppError,
        request::{client_ip, user_agent},
    },
};

#[cfg(test)]
#[path = "identity.domains.service_accounts.routes.tests.rs"]
mod tests;

pub fn router(_state: &AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/workspaces/{workspaceId}/service-accounts",
            get(list_service_accounts).post(create_service_account),
        )
        .route(
            "/workspaces/{workspaceId}/service-accounts/{serviceAccountId}",
            get(get_service_account).patch(update_service_account),
        )
        .route(
            "/workspaces/{workspaceId}/service-accounts/{serviceAccountId}/suspend",
            post(suspend_service_account),
        )
        .route(
            "/workspaces/{workspaceId}/service-accounts/{serviceAccountId}/reactivate",
            post(reactivate_service_account),
        )
        .route(
            "/workspaces/{workspaceId}/service-accounts/{serviceAccountId}/oauth-clients",
            post(create_oauth_client),
        )
        .route(
            "/workspaces/{workspaceId}/service-accounts/{serviceAccountId}/oauth-clients:attach",
            post(attach_oauth_client),
        )
        .route(
            "/workspaces/{workspaceId}/service-accounts/{serviceAccountId}/oauth-clients/{clientId}/rotate-secret",
            post(rotate_oauth_client_secret),
        )
        .route(
            "/workspaces/{workspaceId}/service-accounts/{serviceAccountId}/oauth-clients/{clientId}",
            delete(revoke_oauth_client),
        )
}

#[utoipa::path(
    get,
    path = "/workspaces/{workspaceId}/service-accounts",
    tag = "service-accounts",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
    ),
    responses(
        (status = 200, description = "List workspace service accounts", body = ServiceAccountsResult),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 403, description = "Forbidden", body = ErrorEnvelope),
    ),
)]
pub async fn list_service_accounts(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<ServiceAccountsResult>, AppError> {
    let access = authorize_workspace_action(
        &state.db,
        &state.redis,
        &state.jwt,
        &headers,
        workspace_id,
        WorkspaceAction::ViewMembers,
        ResourceContext::default(),
    )
    .await?;

    Ok(Json(
        crate::domains::service_accounts::service::list_service_accounts(&state.db, &access)
            .await?,
    ))
}

#[utoipa::path(
    post,
    path = "/workspaces/{workspaceId}/service-accounts",
    tag = "service-accounts",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
    ),
    request_body = CreateServiceAccountInput,
    responses(
        (status = 200, description = "Service account created", body = ServiceAccountView),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 403, description = "Forbidden", body = ErrorEnvelope),
    ),
)]
pub async fn create_service_account(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<CreateServiceAccountInput>,
) -> Result<Json<ServiceAccountView>, AppError> {
    let access = authorize_workspace_action(
        &state.db,
        &state.redis,
        &state.jwt,
        &headers,
        workspace_id,
        WorkspaceAction::UpdateWorkspaceSettings,
        ResourceContext::default(),
    )
    .await?;

    Ok(Json(
        crate::domains::service_accounts::service::create_service_account(
            &state.db,
            &access,
            request,
            client_ip(&headers).as_deref(),
            user_agent(&headers).as_deref(),
        )
        .await?,
    ))
}

#[utoipa::path(
    get,
    path = "/workspaces/{workspaceId}/service-accounts/{serviceAccountId}",
    tag = "service-accounts",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
        ("serviceAccountId" = Uuid, Path, description = "Service account principal ID"),
    ),
    responses(
        (status = 200, description = "Service account details", body = ServiceAccountView),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 403, description = "Forbidden", body = ErrorEnvelope),
        (status = 404, description = "Not found", body = ErrorEnvelope),
    ),
)]
pub async fn get_service_account(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, service_account_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<ServiceAccountView>, AppError> {
    let access = authorize_workspace_action(
        &state.db,
        &state.redis,
        &state.jwt,
        &headers,
        workspace_id,
        WorkspaceAction::ViewMembers,
        ResourceContext::default(),
    )
    .await?;

    Ok(Json(
        crate::domains::service_accounts::service::get_service_account(
            &state.db,
            &access,
            service_account_id,
        )
        .await?,
    ))
}

#[utoipa::path(
    patch,
    path = "/workspaces/{workspaceId}/service-accounts/{serviceAccountId}",
    tag = "service-accounts",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
        ("serviceAccountId" = Uuid, Path, description = "Service account principal ID"),
    ),
    request_body = UpdateServiceAccountInput,
    responses(
        (status = 200, description = "Service account updated", body = ServiceAccountView),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 403, description = "Forbidden", body = ErrorEnvelope),
        (status = 404, description = "Not found", body = ErrorEnvelope),
    ),
)]
pub async fn update_service_account(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, service_account_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<UpdateServiceAccountInput>,
) -> Result<Json<ServiceAccountView>, AppError> {
    let access = authorize_workspace_action(
        &state.db,
        &state.redis,
        &state.jwt,
        &headers,
        workspace_id,
        WorkspaceAction::UpdateWorkspaceSettings,
        ResourceContext::default(),
    )
    .await?;

    Ok(Json(
        crate::domains::service_accounts::service::update_service_account(
            &state.db,
            &access,
            service_account_id,
            request,
            client_ip(&headers).as_deref(),
            user_agent(&headers).as_deref(),
        )
        .await?,
    ))
}

#[utoipa::path(
    post,
    path = "/workspaces/{workspaceId}/service-accounts/{serviceAccountId}/suspend",
    tag = "service-accounts",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
        ("serviceAccountId" = Uuid, Path, description = "Service account principal ID"),
    ),
    responses(
        (status = 200, description = "Service account suspended", body = ServiceAccountView),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 403, description = "Forbidden", body = ErrorEnvelope),
        (status = 404, description = "Not found", body = ErrorEnvelope),
    ),
)]
pub async fn suspend_service_account(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, service_account_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<ServiceAccountView>, AppError> {
    let access = authorize_workspace_action(
        &state.db,
        &state.redis,
        &state.jwt,
        &headers,
        workspace_id,
        WorkspaceAction::UpdateWorkspaceSettings,
        ResourceContext::default(),
    )
    .await?;

    let result = crate::domains::service_accounts::service::suspend_service_account(
        &state.db,
        &access,
        service_account_id,
        client_ip(&headers).as_deref(),
        user_agent(&headers).as_deref(),
    )
    .await?;

    let _ =
        nvbes_redis::pubsub::publish_user_suspended(&state.redis, &service_account_id.to_string())
            .await;

    Ok(Json(result))
}

#[utoipa::path(
    post,
    path = "/workspaces/{workspaceId}/service-accounts/{serviceAccountId}/reactivate",
    tag = "service-accounts",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
        ("serviceAccountId" = Uuid, Path, description = "Service account principal ID"),
    ),
    responses(
        (status = 200, description = "Service account reactivated", body = ServiceAccountView),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 403, description = "Forbidden", body = ErrorEnvelope),
        (status = 404, description = "Not found", body = ErrorEnvelope),
    ),
)]
pub async fn reactivate_service_account(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, service_account_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<ServiceAccountView>, AppError> {
    let access = authorize_workspace_action(
        &state.db,
        &state.redis,
        &state.jwt,
        &headers,
        workspace_id,
        WorkspaceAction::UpdateWorkspaceSettings,
        ResourceContext::default(),
    )
    .await?;

    Ok(Json(
        crate::domains::service_accounts::service::reactivate_service_account(
            &state.db,
            &access,
            service_account_id,
            client_ip(&headers).as_deref(),
            user_agent(&headers).as_deref(),
        )
        .await?,
    ))
}

#[utoipa::path(
    post,
    path = "/workspaces/{workspaceId}/service-accounts/{serviceAccountId}/oauth-clients",
    tag = "service-accounts",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
        ("serviceAccountId" = Uuid, Path, description = "Service account principal ID"),
    ),
    request_body = CreateServiceAccountOAuthClientInput,
    responses(
        (status = 200, description = "OAuth client created for the service account", body = CreateServiceAccountOAuthClientResult),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 403, description = "Forbidden", body = ErrorEnvelope),
        (status = 404, description = "Not found", body = ErrorEnvelope),
    ),
)]
pub async fn create_oauth_client(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, service_account_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<CreateServiceAccountOAuthClientInput>,
) -> Result<Json<CreateServiceAccountOAuthClientResult>, AppError> {
    let access = authorize_workspace_action(
        &state.db,
        &state.redis,
        &state.jwt,
        &headers,
        workspace_id,
        WorkspaceAction::UpdateWorkspaceSettings,
        ResourceContext::default(),
    )
    .await?;

    Ok(Json(
        crate::domains::service_accounts::service::create_service_account_oauth_client(
            &state.db,
            &access,
            service_account_id,
            request,
        )
        .await?,
    ))
}

#[utoipa::path(
    post,
    path = "/workspaces/{workspaceId}/service-accounts/{serviceAccountId}/oauth-clients:attach",
    tag = "service-accounts",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
        ("serviceAccountId" = Uuid, Path, description = "Service account principal ID"),
    ),
    request_body = AttachOAuthClientInput,
    responses(
        (status = 200, description = "OAuth client attached", body = ServiceAccountView),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 403, description = "Forbidden", body = ErrorEnvelope),
        (status = 404, description = "Not found", body = ErrorEnvelope),
    ),
)]
pub async fn attach_oauth_client(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, service_account_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<AttachOAuthClientInput>,
) -> Result<Json<ServiceAccountView>, AppError> {
    let access = authorize_workspace_action(
        &state.db,
        &state.redis,
        &state.jwt,
        &headers,
        workspace_id,
        WorkspaceAction::UpdateWorkspaceSettings,
        ResourceContext::default(),
    )
    .await?;

    Ok(Json(
        crate::domains::service_accounts::service::attach_oauth_client(
            &state.db,
            &access,
            service_account_id,
            &request.client_id,
            client_ip(&headers).as_deref(),
            user_agent(&headers).as_deref(),
        )
        .await?,
    ))
}

#[utoipa::path(
    post,
    path = "/workspaces/{workspaceId}/service-accounts/{serviceAccountId}/oauth-clients/{clientId}/rotate-secret",
    tag = "service-accounts",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
        ("serviceAccountId" = Uuid, Path, description = "Service account principal ID"),
        ("clientId" = String, Path, description = "OAuth client ID"),
    ),
    responses(
        (status = 200, description = "OAuth client secret rotated", body = RotateOAuthClientSecretResult),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 403, description = "Forbidden", body = ErrorEnvelope),
        (status = 404, description = "Not found", body = ErrorEnvelope),
    ),
)]
pub async fn rotate_oauth_client_secret(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, service_account_id, client_id)): Path<(Uuid, Uuid, String)>,
) -> Result<Json<RotateOAuthClientSecretResult>, AppError> {
    let access = authorize_workspace_action(
        &state.db,
        &state.redis,
        &state.jwt,
        &headers,
        workspace_id,
        WorkspaceAction::UpdateWorkspaceSettings,
        ResourceContext::default(),
    )
    .await?;

    Ok(Json(
        crate::domains::service_accounts::service::rotate_oauth_client_secret(
            &state.db,
            &access,
            service_account_id,
            &client_id,
            client_ip(&headers).as_deref(),
            user_agent(&headers).as_deref(),
        )
        .await?,
    ))
}

#[utoipa::path(
    delete,
    path = "/workspaces/{workspaceId}/service-accounts/{serviceAccountId}/oauth-clients/{clientId}",
    tag = "service-accounts",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
        ("serviceAccountId" = Uuid, Path, description = "Service account principal ID"),
        ("clientId" = String, Path, description = "OAuth client ID"),
    ),
    responses(
        (status = 200, description = "OAuth client revoked and detached", body = ServiceAccountView),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 403, description = "Forbidden", body = ErrorEnvelope),
        (status = 404, description = "Not found", body = ErrorEnvelope),
    ),
)]
pub async fn revoke_oauth_client(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, service_account_id, client_id)): Path<(Uuid, Uuid, String)>,
) -> Result<Json<ServiceAccountView>, AppError> {
    let access = authorize_workspace_action(
        &state.db,
        &state.redis,
        &state.jwt,
        &headers,
        workspace_id,
        WorkspaceAction::UpdateWorkspaceSettings,
        ResourceContext::default(),
    )
    .await?;

    Ok(Json(
        crate::domains::service_accounts::service::revoke_oauth_client(
            &state.db,
            &state.redis,
            &access,
            service_account_id,
            &client_id,
            client_ip(&headers).as_deref(),
            user_agent(&headers).as_deref(),
        )
        .await?,
    ))
}
