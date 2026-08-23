use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
};
use nvbes_core::http::etag::{if_match, insert_etag, not_modified_if_none_match, resource_etag};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    app::AppState,
    domains::authz::{
        ResourceContext, WorkspaceAction, authorize_workspace_action, ensure_email_verified,
    },
    http::{
        error::AppError,
        request::{client_ip, product_analytics_correlation, user_agent},
    },
};

use super::service::{
    CreateWorkspaceInput, UpdateWorkspaceInput, UpdateWorkspacePolicyInput, WorkspaceListResponse,
    WorkspaceResponse,
};

pub fn router(_state: &AppState) -> Router<AppState> {
    Router::new()
        .route("/workspaces", get(list_workspaces).post(create_workspace))
        .route(
            "/workspaces/{workspaceId}",
            get(get_workspace).patch(update_workspace),
        )
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CreateWorkspaceRequest {
    pub id: Option<Uuid>,
    pub tenant_id: Option<Uuid>,
    pub name: String,
    pub workspace_type: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UpdateWorkspaceRequest {
    pub name: Option<String>,
    pub policy: Option<UpdateWorkspacePolicyRequest>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UpdateWorkspacePolicyRequest {
    pub member_can_create_share_links: Option<bool>,
    pub require_admin_approval_for_member_share: Option<bool>,
    pub default_share_link_ttl_days: Option<i32>,
    pub max_share_link_ttl_days: Option<i32>,
}

async fn list_workspaces(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<WorkspaceListResponse>, AppError> {
    let token = crate::http::request::bearer_token(&headers)?;
    let auth = crate::domains::auth::authenticate(&state.db, &token, Some(&headers)).await?;
    ensure_email_verified(&auth)?;

    Ok(Json(
        crate::domains::workspaces::list_workspaces(&state.db, &auth).await?,
    ))
}

async fn create_workspace(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CreateWorkspaceRequest>,
) -> Result<Response, AppError> {
    let token = crate::http::request::bearer_token(&headers)?;
    let auth = crate::domains::auth::authenticate(&state.db, &token, Some(&headers)).await?;
    ensure_email_verified(&auth)?;

    let (workspace_id, tenant_id) = match (request.id, request.tenant_id) {
        (Some(id), Some(t_id)) => (id, t_id),
        _ => {
            let identity_client = crate::domains::auth::identity::IdentityAuthClient::from_env()?;
            let identity_response = identity_client
                .create_workspace(&token, &request.name, Some(&headers))
                .await?;

            let ws_id_str = identity_response
                .pointer("/workspace/id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    AppError::internal(
                        "invalid_identity_response",
                        "Failed to retrieve workspace ID from Identity response.",
                    )
                })?;
            let ws_id = Uuid::parse_str(ws_id_str).map_err(|_| {
                AppError::internal(
                    "invalid_identity_response",
                    "Failed to parse workspace ID from Identity response.",
                )
            })?;

            let t_id = auth.tenant_id.ok_or_else(|| {
                AppError::internal("missing_tenant", "Tenant ID is missing from AuthContext.")
            })?;

            (ws_id, t_id)
        }
    };

    let result = crate::domains::workspaces::create_workspace(
        &state.db,
        &auth,
        CreateWorkspaceInput {
            id: workspace_id,
            tenant_id,
            name: request.name,
            workspace_type: request.workspace_type,
        },
        client_ip(&headers),
        user_agent(&headers),
    )
    .await?;

    let (distinct_id, session_id) = product_analytics_correlation(&headers);
    state.product_analytics.capture(
        nvbes_product_analytics::ProductAnalyticsEvent::workspace_for_user(
            "workspace.created",
            auth.user_id,
            result.workspace.id,
        )
        .correlation(distinct_id, session_id)
        .property("workspace_type", result.workspace.workspace_type.clone())
        .property("plan_code", result.workspace.plan_code.clone()),
    );

    workspace_response(result)
}

async fn get_workspace(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
) -> Result<Response, AppError> {
    let access = authorize_workspace_action(
        &state.db,
        &headers,
        workspace_id,
        WorkspaceAction::ViewWorkspace,
        ResourceContext::default(),
    )
    .await?;

    let result = crate::domains::workspaces::get_workspace(&state.db, &access).await?;
    let etag = resource_etag(
        "workspace",
        result.workspace.id,
        result.workspace.updated_at,
    );
    if not_modified_if_none_match(&headers, &etag)?.is_some() {
        let mut response = StatusCode::NOT_MODIFIED.into_response();
        insert_etag(response.headers_mut(), &etag)?;
        return Ok(response);
    }
    workspace_response(result)
}

async fn update_workspace(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<UpdateWorkspaceRequest>,
) -> Result<Response, AppError> {
    let access = authorize_workspace_action(
        &state.db,
        &headers,
        workspace_id,
        WorkspaceAction::UpdateWorkspaceSettings,
        ResourceContext::default(),
    )
    .await?;

    workspace_response(
        crate::domains::workspaces::update_workspace(
            &state.db,
            &access,
            UpdateWorkspaceInput {
                name: request.name,
                policy: request.policy.map(|policy| UpdateWorkspacePolicyInput {
                    member_can_create_share_links: policy.member_can_create_share_links,
                    require_admin_approval_for_member_share: policy
                        .require_admin_approval_for_member_share,
                    default_share_link_ttl_days: policy.default_share_link_ttl_days,
                    max_share_link_ttl_days: policy.max_share_link_ttl_days,
                }),
            },
            if_match(&headers)?,
            client_ip(&headers),
            user_agent(&headers),
        )
        .await?,
    )
}

fn workspace_response(result: WorkspaceResponse) -> Result<Response, AppError> {
    let etag = resource_etag(
        "workspace",
        result.workspace.id,
        result.workspace.updated_at,
    );
    let mut headers = HeaderMap::new();
    insert_etag(&mut headers, &etag)?;
    Ok((headers, Json(result)).into_response())
}
