use std::collections::HashSet;

use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::Deserialize;
use sqlx::Row;
use uuid::Uuid;

use crate::{
    access,
    app::DeveloperAppState,
    grpc::{
        activity_logs,
        pb::nvbes::developer::v1::{ListActivityLogsRequest, ListActivityLogsResponse},
    },
    http::{
        auth::{DeveloperAuth, require_recent_step_up},
        context::{optional_string, request_context, time},
        error::AppError,
        types::{
            CreateDeveloperAppRequest, CreateDeveloperAppResponse, DeveloperAppView,
            DeveloperAppsResponse, DeveloperLogEntry, DeveloperLogsResponse, DeveloperMeResponse,
            UpdateDeveloperRedirectsRequest,
        },
    },
    identity::{CreateIdentityOAuthClient, IdentityOAuthClient},
    rbac::{DeveloperPermission, permissions_for_role},
};

const DEVELOPER_LOG_LIMIT: i64 = 100;

#[utoipa::path(get, path = "/developer/me", tag = "developer", responses((status = 200, description = "Developer access")))]
pub async fn me(
    State(state): State<DeveloperAppState>,
    Extension(auth): Extension<DeveloperAuth>,
) -> Result<Json<DeveloperMeResponse>, AppError> {
    let roles = access::roles(&state.db, &auth).await?;
    let mut seen = HashSet::new();
    let permissions = roles
        .iter()
        .copied()
        .flat_map(permissions_for_role)
        .filter(|permission| seen.insert(*permission))
        .collect();

    Ok(Json(DeveloperMeResponse::new(
        auth.tenant_id,
        roles,
        permissions,
    )))
}

#[utoipa::path(get, path = "/developer/apps", tag = "developer", responses((status = 200, description = "Developer apps")))]
pub async fn list_apps(
    State(state): State<DeveloperAppState>,
    Extension(auth): Extension<DeveloperAuth>,
) -> Result<Json<DeveloperAppsResponse>, AppError> {
    access::require_permission(&state.db, &auth, DeveloperPermission::AppsRead).await?;
    let apps = state
        .identity
        .list_oauth_clients(&auth.access_token)
        .await?
        .clients
        .into_iter()
        .map(app_view)
        .collect();
    Ok(Json(DeveloperAppsResponse { apps }))
}

#[utoipa::path(post, path = "/developer/apps", tag = "developer", responses((status = 200, description = "Developer app created")))]
pub async fn create_app(
    State(state): State<DeveloperAppState>,
    Extension(auth): Extension<DeveloperAuth>,
    Json(request): Json<CreateDeveloperAppRequest>,
) -> Result<Json<CreateDeveloperAppResponse>, AppError> {
    access::require_permission(&state.db, &auth, DeveloperPermission::AppsCreate).await?;
    let created = state
        .identity
        .create_oauth_client(
            &auth.access_token,
            &CreateIdentityOAuthClient {
                name: &request.name,
                redirect_uris: &request.redirect_uris,
                allowed_scopes: &request.allowed_scopes,
                allowed_audiences: &request.allowed_audiences,
                allowed_resources: &request.allowed_resources,
                required_acr: "aal1",
                client_type: "confidential",
                owner_scope_type: "tenant",
                owner_scope_id: auth.tenant_id,
                client_assertion_required: false,
                requires_admin_consent: false,
            },
        )
        .await?;

    Ok(Json(CreateDeveloperAppResponse {
        app: app_view(created.client),
        client_secret: created.client_secret,
    }))
}

#[utoipa::path(get, path = "/developer/apps/{clientId}", tag = "developer", responses((status = 200, description = "Developer app")))]
pub async fn get_app(
    State(state): State<DeveloperAppState>,
    Extension(auth): Extension<DeveloperAuth>,
    Path(client_id): Path<String>,
) -> Result<Json<DeveloperAppView>, AppError> {
    access::require_permission(&state.db, &auth, DeveloperPermission::AppsRead).await?;
    let client = state
        .identity
        .list_oauth_clients(&auth.access_token)
        .await?
        .clients
        .into_iter()
        .find(|client| client.client_id == client_id)
        .ok_or_else(|| AppError::not_found("oauth_client_not_found", "OAuth client not found."))?;
    Ok(Json(app_view(client)))
}

#[utoipa::path(patch, path = "/developer/apps/{clientId}/redirects", tag = "developer", responses((status = 200, description = "Developer app redirects updated")))]
pub async fn update_redirects(
    State(state): State<DeveloperAppState>,
    Extension(auth): Extension<DeveloperAuth>,
    Path(client_id): Path<String>,
    Json(request): Json<UpdateDeveloperRedirectsRequest>,
) -> Result<Json<DeveloperAppView>, AppError> {
    access::require_permission(&state.db, &auth, DeveloperPermission::AppsUpdateRedirects).await?;
    require_recent_step_up(&auth)?;
    if request.redirect_uris.is_empty() {
        return Err(AppError::bad_request(
            "validation_failed",
            "At least one redirect URI is required.",
        ));
    }

    let row = sqlx::query(
        r#"
        UPDATE oauth_clients
        SET redirect_uris = $3, updated_at = NOW()
        WHERE tenant_id = $1 AND client_id = $2 AND revoked_at IS NULL
        RETURNING id, client_id, name, redirect_uris, client_type::text AS client_type,
                  created_at, last_used_at
        "#,
    )
    .bind(auth.tenant_id)
    .bind(client_id)
    .bind(request.redirect_uris)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::not_found("oauth_client_not_found", "OAuth client not found."))?;

    Ok(Json(DeveloperAppView {
        id: row.get("id"),
        client_id: row.get("client_id"),
        name: row.get("name"),
        redirect_uris: row.get("redirect_uris"),
        client_type: row.get("client_type"),
        created_at: row.get("created_at"),
        last_used_at: row.get("last_used_at"),
    }))
}

#[utoipa::path(delete, path = "/developer/apps/{clientId}", tag = "developer", responses((status = 204, description = "Developer app revoked")))]
pub async fn revoke_app(
    State(state): State<DeveloperAppState>,
    Extension(auth): Extension<DeveloperAuth>,
    Path(client_id): Path<String>,
) -> Result<StatusCode, AppError> {
    access::require_permission(&state.db, &auth, DeveloperPermission::AppsRevoke).await?;
    state
        .identity
        .revoke_oauth_client(&auth.access_token, &client_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct DeveloperLogsQuery {
    user_id: Option<String>,
    client_id: Option<String>,
    tenant_id: Option<String>,
    event_type: Option<String>,
}

#[utoipa::path(get, path = "/developer/logs", tag = "developer", responses((status = 200, description = "Developer activity logs")))]
pub async fn list_logs(
    State(state): State<DeveloperAppState>,
    Extension(auth): Extension<DeveloperAuth>,
    Query(query): Query<DeveloperLogsQuery>,
) -> Result<Json<DeveloperLogsResponse>, AppError> {
    access::require_permission(&state.db, &auth, DeveloperPermission::LogsRead).await?;
    if query
        .tenant_id
        .as_deref()
        .map(Uuid::parse_str)
        .transpose()
        .map_err(|_| AppError::bad_request("validation_failed", "tenant_id must be a UUID."))?
        .is_some_and(|tenant_id| tenant_id != auth.tenant_id)
    {
        return Ok(Json(DeveloperLogsResponse { logs: Vec::new() }));
    }

    let response = activity_logs::list_activity_logs(
        &state.db,
        ListActivityLogsRequest {
            context: Some(request_context(&auth)),
            tenant_id: auth.tenant_id.to_string(),
            user_id: query.user_id.unwrap_or_default(),
            client_id: query.client_id.unwrap_or_default(),
            event_type: query.event_type.unwrap_or_default(),
            limit: DEVELOPER_LOG_LIMIT,
        },
    )
    .await?;
    Ok(Json(activity_logs_response(response)?))
}

fn activity_logs_response(
    response: ListActivityLogsResponse,
) -> Result<DeveloperLogsResponse, AppError> {
    let logs = response
        .logs
        .into_iter()
        .map(|log| {
            Ok(DeveloperLogEntry {
                id: log.id,
                event_type: log.event_type,
                user_id: optional_string(log.user_id),
                client_id: optional_string(log.client_id),
                tenant_id: optional_string(log.tenant_id),
                created_at: time(&log.created_at, "created_at")?,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    Ok(DeveloperLogsResponse { logs })
}

fn app_view(client: IdentityOAuthClient) -> DeveloperAppView {
    DeveloperAppView {
        id: client.id,
        client_id: client.client_id,
        name: client.name,
        redirect_uris: client.redirect_uris,
        client_type: client.client_type,
        created_at: client.created_at,
        last_used_at: client.last_used_at,
    }
}
