use axum::{
    Extension, Json,
    extract::{Path, State},
};
use uuid::Uuid;

use crate::{
    app::AppState,
    domains::developer::{
        grpc, service,
        types::{
            DeveloperConsoleLogsResponse, DeveloperWebhookDeliveriesResponse,
            DeveloperWebhookDeliverySummary, DeveloperWebhookEndpointsResponse,
        },
    },
    http::{error::AppError, middleware::jwt::AuthContext},
};

pub async fn list_webhooks(
    State(_state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<DeveloperWebhookEndpointsResponse>, AppError> {
    let tenant_id = service::require_tenant_id(&auth)?;
    Ok(Json(
        grpc::list_console_webhook_endpoints(tenant_id, auth.user_id).await?,
    ))
}

pub async fn list_deliveries(
    State(_state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(endpoint_id): Path<Uuid>,
) -> Result<Json<DeveloperWebhookDeliveriesResponse>, AppError> {
    let tenant_id = service::require_tenant_id(&auth)?;
    Ok(Json(
        grpc::list_webhook_deliveries(tenant_id, auth.user_id, endpoint_id).await?,
    ))
}

pub async fn replay_delivery(
    State(_state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(delivery_id): Path<Uuid>,
) -> Result<Json<DeveloperWebhookDeliverySummary>, AppError> {
    let tenant_id = service::require_tenant_id(&auth)?;
    Ok(Json(
        grpc::replay_webhook_delivery(tenant_id, auth.user_id, delivery_id).await?,
    ))
}

pub async fn list_logs(
    State(_state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<DeveloperConsoleLogsResponse>, AppError> {
    let tenant_id = service::require_tenant_id(&auth)?;
    Ok(Json(grpc::list_api_logs(tenant_id, auth.user_id).await?))
}
