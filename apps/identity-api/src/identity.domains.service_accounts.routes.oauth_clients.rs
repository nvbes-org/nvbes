use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use nvbes_core::http::error::ErrorEnvelope;
use uuid::Uuid;

use crate::{
    app::AppState,
    domains::{
        authz::{ResourceContext, WorkspaceAction, authorize_workspace_action},
        service_accounts::types::{
            AttachOAuthClientInput, CreateServiceAccountOAuthClientInput,
            CreateServiceAccountOAuthClientResult, RotateOAuthClientSecretResult,
            ServiceAccountView,
        },
    },
    http::{
        error::AppError,
        request::{client_ip, user_agent},
    },
};

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
