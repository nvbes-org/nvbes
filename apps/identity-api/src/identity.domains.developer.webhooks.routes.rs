use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    routing::{delete, get, post},
};
use nvbes_core::http::error::ErrorEnvelope;
use uuid::Uuid;

use crate::{
    app::AppState,
    domains::developer::{
        rbac::{self, DeveloperPermission},
        rbac_db,
        types::{
            CreateDeveloperWebhookEndpointRequest, CreateDeveloperWebhookEndpointResponse,
            DeveloperWebhooksResponse,
        },
        webhooks_service,
    },
    http::{
        error::AppError,
        middleware::jwt::{AuthContext, jwt_auth_middleware},
    },
};

pub fn router(state: &AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/developer/webhooks",
            get(list_webhooks)
                .post(create_webhook)
                .layer(axum::middleware::from_fn_with_state(
                    state.clone(),
                    jwt_auth_middleware,
                )),
        )
        .route(
            "/developer/webhooks/{endpointId}",
            delete(delete_webhook).layer(axum::middleware::from_fn_with_state(
                state.clone(),
                jwt_auth_middleware,
            )),
        )
}

#[utoipa::path(
    get,
    path = "/developer/webhooks",
    tag = "developer",
    responses(
        (status = 200, description = "Developer webhook endpoints", body = DeveloperWebhooksResponse),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 403, description = "Developer webhook read permission required", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub async fn list_webhooks(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<DeveloperWebhooksResponse>, AppError> {
    let tenant_id =
        require_developer_permission(&state, &auth, DeveloperPermission::WebhooksRead).await?;
    let response = webhooks_service::list_endpoints(&state.db, tenant_id).await?;

    Ok(Json(response))
}

#[utoipa::path(
    post,
    path = "/developer/webhooks",
    tag = "developer",
    request_body = CreateDeveloperWebhookEndpointRequest,
    responses(
        (status = 200, description = "Developer webhook endpoint created", body = CreateDeveloperWebhookEndpointResponse),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 403, description = "Developer webhook manage permission required", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub async fn create_webhook(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(request): Json<CreateDeveloperWebhookEndpointRequest>,
) -> Result<Json<CreateDeveloperWebhookEndpointResponse>, AppError> {
    let tenant_id =
        require_developer_permission(&state, &auth, DeveloperPermission::WebhooksManage).await?;
    let response =
        webhooks_service::create_endpoint(&state.db, tenant_id, auth.user_id, request).await?;

    Ok(Json(response))
}

#[utoipa::path(
    delete,
    path = "/developer/webhooks/{endpointId}",
    tag = "developer",
    params(
        ("endpointId" = Uuid, Path, description = "Developer webhook endpoint ID"),
    ),
    responses(
        (status = 204, description = "Developer webhook endpoint revoked"),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 403, description = "Developer webhook manage permission required", body = ErrorEnvelope),
        (status = 404, description = "Not found", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub async fn delete_webhook(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(endpoint_id): Path<Uuid>,
) -> Result<axum::http::StatusCode, AppError> {
    let tenant_id =
        require_developer_permission(&state, &auth, DeveloperPermission::WebhooksManage).await?;
    webhooks_service::delete_endpoint(&state.db, tenant_id, endpoint_id).await?;

    Ok(axum::http::StatusCode::NO_CONTENT)
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
