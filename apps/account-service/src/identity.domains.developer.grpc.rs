use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use tonic::{Code, transport::Channel};
use uuid::Uuid;

use crate::{
    domains::developer::types::{
        CreateDeveloperWebhookEndpointRequest, CreateDeveloperWebhookEndpointResponse,
        DeveloperConsoleLogEntry, DeveloperConsoleLogsResponse, DeveloperWebhookDeliveriesResponse,
        DeveloperWebhookDeliverySummary, DeveloperWebhookEndpointSummary,
        DeveloperWebhookEndpointView, DeveloperWebhookEndpointsResponse, DeveloperWebhookEventType,
        DeveloperWebhooksResponse,
    },
    grpc_pb::nvbes::{
        developer::v1::{
            CreateWebhookEndpointRequest, DeleteWebhookEndpointRequest, ListApiLogsRequest,
            ListWebhookDeliveriesRequest, ListWebhookEndpointsRequest,
            ReplayWebhookDeliveryRequest, developer_service_client::DeveloperServiceClient,
        },
        platform::v1::{RequestContext, TenantContext},
    },
    http::error::AppError,
};

const DEVELOPER_GRPC_ENDPOINT_ENV: &str = "NVBES_DEVELOPER_GRPC_ENDPOINT";

#[path = "identity.domains.developer.grpc.activity_logs.rs"]
mod activity_logs;
#[path = "identity.domains.developer.grpc.client_metadata.rs"]
mod client_metadata;
#[path = "identity.domains.developer.grpc.credentials.rs"]
mod credentials;
#[path = "identity.domains.developer.grpc.health.rs"]
mod health;
#[path = "identity.domains.developer.grpc.marketplace.rs"]
mod marketplace;
#[path = "identity.domains.developer.grpc.overview.rs"]
mod overview;
#[path = "identity.domains.developer.grpc.rbac.rs"]
mod rbac;
#[path = "identity.domains.developer.grpc.sandbox.rs"]
mod sandbox;
#[path = "identity.domains.developer.grpc.scopes.rs"]
mod scopes;
#[path = "identity.domains.developer.grpc.secrets.rs"]
mod secrets;
#[path = "identity.domains.developer.grpc.tokens.rs"]
mod tokens;

pub use activity_logs::list_activity_logs;
pub use client_metadata::{DeveloperClientMetadata, get_client_metadata};
pub use credentials::{
    DeveloperCredentialSummary, count_stale_secrets, list_credential_summaries,
    verify_client_secret_version,
};
pub use health::{list_health_checks, run_health_checks};
pub use marketplace::{
    DeveloperMarketplaceAppRecord, get_consent_screen, get_public_consent_screen,
    list_marketplace_apps, review_marketplace_app, submit_marketplace_app, upsert_consent_screen,
};
pub use overview::get_overview_summary;
pub use rbac::list_developer_roles;
pub use sandbox::{get_sandbox, reset_sandbox, upsert_sandbox};
pub use scopes::{create_scope, delete_scope, list_scopes, update_scope};
pub use secrets::{list_secret_versions, record_secret_rotation, revoke_secret_version};
pub use tokens::record_token_debug_session;

pub fn developer_grpc_endpoint() -> anyhow::Result<String> {
    match std::env::var(DEVELOPER_GRPC_ENDPOINT_ENV) {
        Ok(endpoint) if !endpoint.trim().is_empty() => Ok(endpoint),
        Ok(_) => anyhow::bail!("{DEVELOPER_GRPC_ENDPOINT_ENV} must not be empty"),
        Err(std::env::VarError::NotPresent) => Ok("http://127.0.0.1:4041".to_string()),
        Err(error) => Err(anyhow::anyhow!(
            "{DEVELOPER_GRPC_ENDPOINT_ENV} could not be read: {error}"
        )),
    }
}

pub async fn list_webhook_endpoints(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
) -> Result<DeveloperWebhooksResponse, AppError> {
    let mut client = developer_client().await?;
    let response = client
        .list_webhook_endpoints(ListWebhookEndpointsRequest {
            context: Some(request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
            page: None,
        })
        .await
        .map_err(grpc_error)?
        .into_inner();

    Ok(DeveloperWebhooksResponse {
        endpoints: response
            .endpoints
            .into_iter()
            .map(webhook_endpoint_view)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

pub async fn create_webhook_endpoint(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    request: CreateDeveloperWebhookEndpointRequest,
) -> Result<CreateDeveloperWebhookEndpointResponse, AppError> {
    let mut client = developer_client().await?;
    let endpoint = client
        .create_webhook_endpoint(CreateWebhookEndpointRequest {
            context: Some(request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
            name: request.name,
            url: request.url,
            event_types: request
                .events
                .into_iter()
                .map(|event| event.as_event_type().to_string())
                .collect(),
        })
        .await
        .map_err(grpc_error)?
        .into_inner();

    let signing_secret = endpoint.signing_secret.clone();
    Ok(CreateDeveloperWebhookEndpointResponse {
        endpoint: webhook_endpoint_view(endpoint)?,
        signing_secret,
    })
}

pub async fn list_console_webhook_endpoints(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
) -> Result<DeveloperWebhookEndpointsResponse, AppError> {
    let mut client = developer_client().await?;
    let response = client
        .list_webhook_endpoints(ListWebhookEndpointsRequest {
            context: Some(request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
            page: None,
        })
        .await
        .map_err(grpc_error)?
        .into_inner();

    Ok(DeveloperWebhookEndpointsResponse {
        webhooks: response
            .endpoints
            .into_iter()
            .map(webhook_endpoint_summary)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

pub async fn list_webhook_deliveries(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    endpoint_id: Uuid,
) -> Result<DeveloperWebhookDeliveriesResponse, AppError> {
    let mut client = developer_client().await?;
    let response = client
        .list_webhook_deliveries(ListWebhookDeliveriesRequest {
            context: Some(request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
            endpoint_id: endpoint_id.to_string(),
            page: None,
        })
        .await
        .map_err(grpc_error)?
        .into_inner();

    Ok(DeveloperWebhookDeliveriesResponse {
        deliveries: response
            .deliveries
            .into_iter()
            .map(webhook_delivery_summary)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

pub async fn replay_webhook_delivery(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    delivery_id: Uuid,
) -> Result<DeveloperWebhookDeliverySummary, AppError> {
    let mut client = developer_client().await?;
    let delivery = client
        .replay_webhook_delivery(ReplayWebhookDeliveryRequest {
            context: Some(request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
            delivery_id: delivery_id.to_string(),
        })
        .await
        .map_err(grpc_error)?
        .into_inner();

    webhook_delivery_summary(delivery)
}

pub async fn list_api_logs(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
) -> Result<DeveloperConsoleLogsResponse, AppError> {
    let mut client = developer_client().await?;
    let response = client
        .list_api_logs(ListApiLogsRequest {
            context: Some(request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
            client_id: String::new(),
            page: None,
        })
        .await
        .map_err(grpc_error)?
        .into_inner();

    Ok(DeveloperConsoleLogsResponse {
        logs: response
            .logs
            .into_iter()
            .map(api_log_entry)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

pub async fn delete_webhook_endpoint(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    endpoint_id: Uuid,
) -> Result<(), AppError> {
    let mut client = developer_client().await?;
    client
        .delete_webhook_endpoint(DeleteWebhookEndpointRequest {
            context: Some(request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
            endpoint_id: endpoint_id.to_string(),
        })
        .await
        .map_err(grpc_error)?;
    Ok(())
}

async fn developer_client() -> Result<DeveloperServiceClient<Channel>, AppError> {
    let endpoint = developer_grpc_endpoint().map_err(|error| {
        AppError::internal("developer_grpc_endpoint_invalid", error.to_string())
    })?;
    DeveloperServiceClient::connect(endpoint)
        .await
        .map_err(|error| AppError::internal("developer_grpc_connect_failed", error.to_string()))
}

fn request_context(tenant_id: Uuid, actor_principal_id: Uuid) -> RequestContext {
    RequestContext {
        request_id: Uuid::new_v4().to_string(),
        correlation_id: Uuid::new_v4().to_string(),
        actor_principal_id: actor_principal_id.to_string(),
        tenant: Some(TenantContext {
            tenant_id: tenant_id.to_string(),
            workspace_id: String::new(),
            region_id: String::new(),
            data_residency: String::new(),
        }),
    }
}

fn global_request_context(actor_principal_id: Uuid) -> RequestContext {
    RequestContext {
        request_id: Uuid::new_v4().to_string(),
        correlation_id: Uuid::new_v4().to_string(),
        actor_principal_id: actor_principal_id.to_string(),
        tenant: None,
    }
}

fn webhook_endpoint_view(
    endpoint: crate::grpc_pb::nvbes::developer::v1::WebhookEndpoint,
) -> Result<DeveloperWebhookEndpointView, AppError> {
    Ok(DeveloperWebhookEndpointView {
        id: parse_uuid(&endpoint.endpoint_id, "endpoint_id")?,
        name: endpoint.name,
        url: endpoint.url,
        status: endpoint.status,
        events: endpoint
            .event_types
            .iter()
            .map(|event| parse_event(event))
            .collect::<Result<Vec<_>, _>>()?,
        signing_secret_last4: endpoint.signing_secret_last4,
        created_at: parse_time(&endpoint.created_at, "created_at")?,
    })
}

fn webhook_endpoint_summary(
    endpoint: crate::grpc_pb::nvbes::developer::v1::WebhookEndpoint,
) -> Result<DeveloperWebhookEndpointSummary, AppError> {
    Ok(DeveloperWebhookEndpointSummary {
        id: parse_uuid(&endpoint.endpoint_id, "endpoint_id")?,
        name: endpoint.name,
        url: endpoint.url,
        status: endpoint.status,
        failed_delivery_count: endpoint.failed_delivery_count,
        created_at: parse_time(&endpoint.created_at, "created_at")?,
        updated_at: parse_time(&endpoint.updated_at, "updated_at")?,
    })
}

fn webhook_delivery_summary(
    delivery: crate::grpc_pb::nvbes::developer::v1::WebhookDelivery,
) -> Result<DeveloperWebhookDeliverySummary, AppError> {
    Ok(DeveloperWebhookDeliverySummary {
        id: parse_uuid(&delivery.delivery_id, "delivery_id")?,
        endpoint_id: parse_uuid(&delivery.endpoint_id, "endpoint_id")?,
        event_id: parse_uuid(&delivery.event_id, "event_id")?,
        event_type: delivery.event_type,
        status: delivery.status,
        attempt_count: delivery.attempt_count,
        response_status: parse_optional_i32(&delivery.response_status, "response_status")?,
        error_message: empty_to_option(delivery.error_message),
        created_at: parse_time(&delivery.created_at, "created_at")?,
        delivered_at: parse_optional_time(&delivery.delivered_at, "delivered_at")?,
        replayed_from_delivery_id: parse_optional_uuid(
            &delivery.replayed_from_delivery_id,
            "replayed_from_delivery_id",
        )?,
    })
}

fn api_log_entry(
    entry: crate::grpc_pb::nvbes::developer::v1::ApiLogEntry,
) -> Result<DeveloperConsoleLogEntry, AppError> {
    Ok(DeveloperConsoleLogEntry {
        id: parse_uuid(&entry.id, "id")?,
        source: entry.source,
        event_type: entry.event_type,
        severity: entry.severity,
        message: entry.message,
        created_at: parse_time(&entry.created_at, "created_at")?,
    })
}

fn parse_event(value: &str) -> Result<DeveloperWebhookEventType, AppError> {
    match value {
        "user.created" => Ok(DeveloperWebhookEventType::UserCreated),
        "login.failed" => Ok(DeveloperWebhookEventType::LoginFailed),
        "session.revoked" => Ok(DeveloperWebhookEventType::SessionRevoked),
        "client.created" => Ok(DeveloperWebhookEventType::ClientCreated),
        _ => Err(AppError::internal(
            "developer_projection_invalid",
            format!("Unknown webhook event type from Developer service: {value}."),
        )),
    }
}

fn parse_uuid(value: &str, field: &'static str) -> Result<Uuid, AppError> {
    Uuid::parse_str(value.trim()).map_err(|error| {
        AppError::internal(
            "developer_projection_invalid",
            format!("{field} from Developer service is not a valid UUID: {error}"),
        )
    })
}

fn parse_optional_uuid(value: &str, field: &'static str) -> Result<Option<Uuid>, AppError> {
    if value.trim().is_empty() {
        Ok(None)
    } else {
        parse_uuid(value, field).map(Some)
    }
}

fn parse_time(value: &str, field: &'static str) -> Result<DateTime<Utc>, AppError> {
    DateTime::parse_from_rfc3339(value.trim())
        .map(|value| value.with_timezone(&Utc))
        .map_err(|error| {
            AppError::internal(
                "developer_projection_invalid",
                format!("{field} from Developer service is not an RFC3339 timestamp: {error}"),
            )
        })
}

fn parse_optional_time(
    value: &str,
    field: &'static str,
) -> Result<Option<DateTime<Utc>>, AppError> {
    if value.trim().is_empty() {
        Ok(None)
    } else {
        parse_time(value, field).map(Some)
    }
}

fn parse_optional_i32(value: &str, field: &'static str) -> Result<Option<i32>, AppError> {
    if value.trim().is_empty() {
        Ok(None)
    } else {
        value.trim().parse::<i32>().map(Some).map_err(|error| {
            AppError::internal(
                "developer_projection_invalid",
                format!("{field} from Developer service is not a valid integer: {error}"),
            )
        })
    }
}

fn empty_to_option(value: String) -> Option<String> {
    let value = value.trim().to_string();
    if value.is_empty() { None } else { Some(value) }
}

fn grpc_error(error: tonic::Status) -> AppError {
    let status = match error.code() {
        Code::InvalidArgument => StatusCode::BAD_REQUEST,
        Code::Unauthenticated => StatusCode::UNAUTHORIZED,
        Code::PermissionDenied => StatusCode::FORBIDDEN,
        Code::NotFound => StatusCode::NOT_FOUND,
        Code::AlreadyExists | Code::FailedPrecondition => StatusCode::CONFLICT,
        Code::Unavailable | Code::DeadlineExceeded => StatusCode::BAD_GATEWAY,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    AppError::new(status, "developer_grpc_error", error.message().to_string())
}
