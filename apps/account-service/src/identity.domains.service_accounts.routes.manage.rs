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
            CreateServiceAccountInput, ServiceAccountView, ServiceAccountsResult,
            UpdateServiceAccountInput,
        },
    },
    http::{
        error::AppError,
        request::{client_ip, user_agent},
    },
};

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
