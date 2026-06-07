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
        service_accounts::types::ServiceAccountView,
    },
    http::{
        error::AppError,
        request::{client_ip, user_agent},
    },
};

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
