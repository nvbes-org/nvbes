use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::get,
};
use nvbes_core::http::error::ErrorEnvelope;
use serde::Deserialize;
use uuid::Uuid;

use crate::app::AppState;
use crate::domains::auth::sessions;
use crate::domains::authz::{
    ResourceContext, WorkspaceAction, authorize_workspace_action, ensure_email_verified,
};
use crate::domains::workspaces::service::{
    self, CreateWorkspaceInput, UpdateWorkspaceInput, WorkspaceListResponse, WorkspaceResponse,
};
use crate::http::error::AppError;
use crate::http::request::{bearer_token, client_ip, user_agent};
use nvbes_product_analytics::ProductAnalyticsEvent;

pub fn router(_state: &AppState) -> Router<AppState> {
    Router::new()
        .route("/workspaces", get(list_workspaces).post(create_workspace))
        .route(
            "/workspaces/{workspaceId}",
            get(get_workspace).patch(update_workspace),
        )
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) struct CreateWorkspaceRequest {
    name: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) struct UpdateWorkspaceRequest {
    name: String,
}

#[utoipa::path(
    get,
    path = "/workspaces",
    tag = "workspaces",
    responses(
        (status = 200, description = "List workspaces", body = WorkspaceListResponse),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn list_workspaces(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<WorkspaceListResponse>, AppError> {
    let token = bearer_token(&headers)?;
    let auth = sessions::authenticate(&state.db, &state.redis, &state.jwt, &token).await?;
    ensure_email_verified(&auth)?;

    Ok(Json(service::list_workspaces(&state.db, &auth).await?))
}

#[utoipa::path(
    post,
    path = "/workspaces",
    tag = "workspaces",
    request_body = CreateWorkspaceRequest,
    responses(
        (status = 200, description = "Workspace created", body = WorkspaceResponse),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn create_workspace(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CreateWorkspaceRequest>,
) -> Result<Json<WorkspaceResponse>, AppError> {
    let token = bearer_token(&headers)?;
    let auth = sessions::authenticate(&state.db, &state.redis, &state.jwt, &token).await?;
    ensure_email_verified(&auth)?;

    let result = service::create_workspace(
        &state.db,
        &auth,
        CreateWorkspaceInput {
            name: request.name,
            workspace_type: None,
            jurisdiction: None,
            region: None,
        },
        client_ip(&headers),
        user_agent(&headers),
    )
    .await?;

    state.product_analytics.capture(
        ProductAnalyticsEvent::workspace_for_user(
            "workspace.created",
            auth.user_id,
            result.workspace.id,
        )
        .property("workspace_type", result.workspace.workspace_type.clone())
        .property("plan_code", result.workspace.plan_code.clone()),
    );

    Ok(Json(result))
}

#[utoipa::path(
    get,
    path = "/workspaces/{workspaceId}",
    tag = "workspaces",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
    ),
    responses(
        (status = 200, description = "Workspace details", body = WorkspaceResponse),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 404, description = "Not found", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn get_workspace(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<WorkspaceResponse>, AppError> {
    let access = authorize_workspace_action(
        &state.db,
        &state.redis,
        &state.jwt,
        &headers,
        workspace_id,
        WorkspaceAction::ViewWorkspace,
        ResourceContext::default(),
    )
    .await?;

    Ok(Json(service::get_workspace(&state.db, &access).await?))
}

#[utoipa::path(
    patch,
    path = "/workspaces/{workspaceId}",
    tag = "workspaces",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
    ),
    request_body = UpdateWorkspaceRequest,
    responses(
        (status = 200, description = "Workspace updated", body = WorkspaceResponse),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 404, description = "Not found", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn update_workspace(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<UpdateWorkspaceRequest>,
) -> Result<Json<WorkspaceResponse>, AppError> {
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
        service::update_workspace(
            &state.db,
            &access,
            UpdateWorkspaceInput {
                name: Some(request.name),
                policy: None,
            },
            client_ip(&headers),
            user_agent(&headers),
        )
        .await?,
    ))
}
