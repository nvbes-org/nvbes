use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domains::developer::{
        grpc,
        types::{
            CreateDeveloperWebhookEndpointRequest, CreateDeveloperWebhookEndpointResponse,
            DeveloperWebhooksResponse,
        },
    },
    http::error::AppError,
};

pub async fn list_endpoints(
    _db: &PgPool,
    tenant_id: Uuid,
    actor_principal_id: Uuid,
) -> Result<DeveloperWebhooksResponse, AppError> {
    grpc::list_webhook_endpoints(tenant_id, actor_principal_id).await
}

pub async fn create_endpoint(
    _db: &PgPool,
    tenant_id: Uuid,
    created_by: Uuid,
    request: CreateDeveloperWebhookEndpointRequest,
) -> Result<CreateDeveloperWebhookEndpointResponse, AppError> {
    grpc::create_webhook_endpoint(tenant_id, created_by, request).await
}

pub async fn delete_endpoint(
    _db: &PgPool,
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    endpoint_id: Uuid,
) -> Result<(), AppError> {
    grpc::delete_webhook_endpoint(tenant_id, actor_principal_id, endpoint_id).await
}
