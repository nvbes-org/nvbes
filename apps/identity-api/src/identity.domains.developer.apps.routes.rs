use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    routing::{get, patch},
};
use nvbes_core::http::error::ErrorEnvelope;
use uuid::Uuid;

use crate::{
    app::AppState,
    domains::{
        auth::verification,
        developer::{
            apps_service,
            rbac::{self, DeveloperPermission},
            rbac_db,
            types::{
                CreateDeveloperAppRequest, CreateDeveloperAppResponse, DeveloperAppView,
                DeveloperAppsResponse, UpdateDeveloperRedirectsRequest,
            },
        },
    },
    http::{
        error::AppError,
        middleware::jwt::{AuthContext, jwt_auth_middleware},
    },
};

pub fn router(state: &AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/developer/apps",
            get(list_apps)
                .post(create_app)
                .layer(axum::middleware::from_fn_with_state(
                    state.clone(),
                    jwt_auth_middleware,
                )),
        )
        .route(
            "/developer/apps/{clientId}",
            get(get_app)
                .delete(revoke_app)
                .layer(axum::middleware::from_fn_with_state(
                    state.clone(),
                    jwt_auth_middleware,
                )),
        )
        .route(
            "/developer/apps/{clientId}/redirects",
            patch(update_redirects).layer(axum::middleware::from_fn_with_state(
                state.clone(),
                jwt_auth_middleware,
            )),
        )
}

#[utoipa::path(
    get,
    path = "/developer/apps",
    tag = "developer",
    responses(
        (status = 200, description = "Developer OAuth apps", body = DeveloperAppsResponse),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 403, description = "Developer app read permission required", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub async fn list_apps(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<DeveloperAppsResponse>, AppError> {
    require_developer_permission(&state, &auth, DeveloperPermission::AppsRead).await?;
    let response = apps_service::list_apps(&state.db, &auth).await?;

    Ok(Json(response))
}

#[utoipa::path(
    post,
    path = "/developer/apps",
    tag = "developer",
    request_body = CreateDeveloperAppRequest,
    responses(
        (status = 200, description = "Developer OAuth app created", body = CreateDeveloperAppResponse),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 403, description = "Developer app create permission required", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub async fn create_app(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(request): Json<CreateDeveloperAppRequest>,
) -> Result<Json<CreateDeveloperAppResponse>, AppError> {
    require_developer_permission(&state, &auth, DeveloperPermission::AppsCreate).await?;
    verification::require_recent_step_up(&state.redis, &auth, None).await?;

    let response = apps_service::create_app(&state.db, &auth, request).await?;
    state
        .allowed_browser_origins
        .refresh_from_db(&state.db, &state.config)
        .await?;

    Ok(Json(response))
}

#[utoipa::path(
    get,
    path = "/developer/apps/{clientId}",
    tag = "developer",
    params(
        ("clientId" = String, Path, description = "OAuth client ID"),
    ),
    responses(
        (status = 200, description = "Developer OAuth app", body = DeveloperAppView),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 403, description = "Developer app read permission required", body = ErrorEnvelope),
        (status = 404, description = "Not found", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub async fn get_app(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(client_id): Path<String>,
) -> Result<Json<DeveloperAppView>, AppError> {
    let tenant_id =
        require_developer_permission(&state, &auth, DeveloperPermission::AppsRead).await?;
    let response = apps_service::get_app(&state.db, tenant_id, &client_id).await?;

    Ok(Json(response))
}

#[utoipa::path(
    patch,
    path = "/developer/apps/{clientId}/redirects",
    tag = "developer",
    params(
        ("clientId" = String, Path, description = "OAuth client ID"),
    ),
    request_body = UpdateDeveloperRedirectsRequest,
    responses(
        (status = 200, description = "Developer OAuth app redirects updated", body = DeveloperAppView),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 403, description = "Developer app redirects permission required", body = ErrorEnvelope),
        (status = 404, description = "Not found", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub async fn update_redirects(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(client_id): Path<String>,
    Json(request): Json<UpdateDeveloperRedirectsRequest>,
) -> Result<Json<DeveloperAppView>, AppError> {
    let tenant_id =
        require_developer_permission(&state, &auth, DeveloperPermission::AppsUpdateRedirects)
            .await?;
    verification::require_recent_step_up(&state.redis, &auth, None).await?;

    let response =
        apps_service::update_redirects(&state.db, tenant_id, &client_id, request).await?;
    state
        .allowed_browser_origins
        .refresh_from_db(&state.db, &state.config)
        .await?;

    Ok(Json(response))
}

#[utoipa::path(
    delete,
    path = "/developer/apps/{clientId}",
    tag = "developer",
    params(
        ("clientId" = String, Path, description = "OAuth client ID"),
    ),
    responses(
        (status = 200, description = "Developer OAuth app revoked", body = crate::domains::oauth::service::RevokeOAuthClientResult),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 403, description = "Developer app revoke permission required", body = ErrorEnvelope),
        (status = 404, description = "Not found", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub async fn revoke_app(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(client_id): Path<String>,
) -> Result<Json<crate::domains::oauth::service::RevokeOAuthClientResult>, AppError> {
    require_developer_permission(&state, &auth, DeveloperPermission::AppsRevoke).await?;
    verification::require_recent_step_up(&state.redis, &auth, None).await?;

    let response = apps_service::revoke_app(&state.db, &state.redis, &auth, &client_id).await?;
    state
        .allowed_browser_origins
        .refresh_from_db(&state.db, &state.config)
        .await?;

    Ok(Json(response))
}

async fn require_developer_permission(
    state: &AppState,
    auth: &AuthContext,
    required: DeveloperPermission,
) -> Result<Uuid, AppError> {
    let tenant_id = auth.tenant_id.ok_or_else(|| {
        AppError::forbidden(
            "tenant_context_required",
            "Tenant context is required for developer portal access.",
        )
    })?;
    let roles = rbac_db::load_developer_roles(&state.db, tenant_id, auth.user_id).await?;
    let permissions = rbac_db::permissions_for_roles(&roles);

    if !rbac::has_permission(&permissions, required) {
        return Err(AppError::forbidden(
            "developer_permission_required",
            format!("Developer permission {} is required.", required.as_scope()),
        ));
    }

    Ok(tenant_id)
}
