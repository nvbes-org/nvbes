use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
};
use uuid::Uuid;

use crate::{
    access,
    app::DeveloperAppState,
    grpc::{pb::nvbes::developer::v1::CreateWebhookEndpointRequest, webhooks},
    http::{
        auth::DeveloperAuth,
        context::{optional_string, optional_time, time, uuid},
        error::AppError,
        types::{
            CreateDeveloperWebhookEndpointRequest, CreateDeveloperWebhookEndpointResponse,
            DeveloperConsoleLogEntry, DeveloperConsoleLogsResponse,
            DeveloperWebhookDeliveriesResponse, DeveloperWebhookDeliverySummary,
            DeveloperWebhookEndpointSummary, DeveloperWebhookEndpointView,
            DeveloperWebhookEndpointsResponse, DeveloperWebhookEventType,
            DeveloperWebhooksResponse,
        },
    },
    rbac::DeveloperPermission,
};

#[utoipa::path(get, path = "/developer/webhooks", tag = "developer", responses((status = 200, description = "Developer webhooks")))]
pub async fn list_portal_webhooks(
    State(state): State<DeveloperAppState>,
    Extension(auth): Extension<DeveloperAuth>,
) -> Result<Json<DeveloperWebhooksResponse>, AppError> {
    access::require_permission(&state.db, &auth, DeveloperPermission::WebhooksRead).await?;
    let response = webhooks::list_webhook_endpoints(&state.db, auth.tenant_id).await?;
    let endpoints = response
        .endpoints
        .into_iter()
        .map(portal_endpoint)
        .collect::<Result<Vec<_>, AppError>>()?;
    Ok(Json(DeveloperWebhooksResponse { endpoints }))
}

#[utoipa::path(post, path = "/developer/webhooks", tag = "developer", responses((status = 200, description = "Developer webhook created")))]
pub async fn create_portal_webhook(
    State(state): State<DeveloperAppState>,
    Extension(auth): Extension<DeveloperAuth>,
    Json(input): Json<CreateDeveloperWebhookEndpointRequest>,
) -> Result<Json<CreateDeveloperWebhookEndpointResponse>, AppError> {
    access::require_permission(&state.db, &auth, DeveloperPermission::WebhooksManage).await?;
    let endpoint = webhooks::create_webhook_endpoint(
        &state.db,
        auth.tenant_id,
        auth.user_id,
        CreateWebhookEndpointRequest {
            context: None,
            tenant_id: auth.tenant_id.to_string(),
            name: input.name,
            url: input.url,
            event_types: input
                .events
                .into_iter()
                .map(|event| event.as_event_type().to_string())
                .collect(),
        },
    )
    .await?;
    let signing_secret = endpoint.signing_secret.clone();
    Ok(Json(CreateDeveloperWebhookEndpointResponse {
        endpoint: portal_endpoint(endpoint)?,
        signing_secret,
    }))
}

#[utoipa::path(delete, path = "/developer/webhooks/{endpointId}", tag = "developer", responses((status = 204, description = "Developer webhook revoked")))]
pub async fn delete_portal_webhook(
    State(state): State<DeveloperAppState>,
    Extension(auth): Extension<DeveloperAuth>,
    Path(endpoint_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    access::require_permission(&state.db, &auth, DeveloperPermission::WebhooksManage).await?;
    webhooks::delete_webhook_endpoint(&state.db, auth.tenant_id, endpoint_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(get, path = "/developer/console/webhooks", tag = "developer-console", responses((status = 200, description = "Console webhooks")))]
pub async fn list_console_webhooks(
    State(state): State<DeveloperAppState>,
    Extension(auth): Extension<DeveloperAuth>,
) -> Result<Json<DeveloperWebhookEndpointsResponse>, AppError> {
    access::require_permission(&state.db, &auth, DeveloperPermission::ConsoleWebhooksRead).await?;
    let response = webhooks::list_webhook_endpoints(&state.db, auth.tenant_id).await?;
    let webhooks = response
        .endpoints
        .into_iter()
        .map(console_endpoint)
        .collect::<Result<Vec<_>, AppError>>()?;
    Ok(Json(DeveloperWebhookEndpointsResponse { webhooks }))
}

#[utoipa::path(get, path = "/developer/console/webhooks/{endpointId}/deliveries", tag = "developer-console", responses((status = 200, description = "Webhook deliveries")))]
pub async fn list_deliveries(
    State(state): State<DeveloperAppState>,
    Extension(auth): Extension<DeveloperAuth>,
    Path(endpoint_id): Path<Uuid>,
) -> Result<Json<DeveloperWebhookDeliveriesResponse>, AppError> {
    access::require_permission(&state.db, &auth, DeveloperPermission::ConsoleWebhooksRead).await?;
    let response =
        webhooks::list_webhook_deliveries(&state.db, auth.tenant_id, endpoint_id).await?;
    let deliveries = response
        .deliveries
        .into_iter()
        .map(delivery)
        .collect::<Result<Vec<_>, AppError>>()?;
    Ok(Json(DeveloperWebhookDeliveriesResponse { deliveries }))
}

#[utoipa::path(post, path = "/developer/console/webhooks/deliveries/{deliveryId}/replay", tag = "developer-console", responses((status = 200, description = "Webhook delivery replayed")))]
pub async fn replay_delivery(
    State(state): State<DeveloperAppState>,
    Extension(auth): Extension<DeveloperAuth>,
    Path(delivery_id): Path<Uuid>,
) -> Result<Json<DeveloperWebhookDeliverySummary>, AppError> {
    access::require_permission(&state.db, &auth, DeveloperPermission::WebhooksReplay).await?;
    let replay = webhooks::replay_webhook_delivery(&state.db, auth.tenant_id, delivery_id).await?;
    Ok(Json(delivery(replay)?))
}

#[utoipa::path(get, path = "/developer/console/logs", tag = "developer-console", responses((status = 200, description = "Developer API logs")))]
pub async fn list_api_logs(
    State(state): State<DeveloperAppState>,
    Extension(auth): Extension<DeveloperAuth>,
) -> Result<Json<DeveloperConsoleLogsResponse>, AppError> {
    access::require_permission(&state.db, &auth, DeveloperPermission::ConsoleLogsRead).await?;
    let response = webhooks::list_api_logs(&state.db, auth.tenant_id).await?;
    let logs = response
        .logs
        .into_iter()
        .map(|log| {
            Ok(DeveloperConsoleLogEntry {
                id: uuid(&log.id, "log id")?,
                source: log.source,
                event_type: log.event_type,
                severity: log.severity,
                message: log.message,
                created_at: time(&log.created_at, "created_at")?,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    Ok(Json(DeveloperConsoleLogsResponse { logs }))
}

fn portal_endpoint(
    endpoint: crate::grpc::pb::nvbes::developer::v1::WebhookEndpoint,
) -> Result<DeveloperWebhookEndpointView, AppError> {
    Ok(DeveloperWebhookEndpointView {
        id: uuid(&endpoint.endpoint_id, "endpoint id")?,
        name: endpoint.name,
        url: endpoint.url,
        status: endpoint.status,
        events: endpoint
            .event_types
            .into_iter()
            .map(parse_event_type)
            .collect::<Result<Vec<_>, AppError>>()?,
        signing_secret_last4: endpoint.signing_secret_last4,
        created_at: time(&endpoint.created_at, "created_at")?,
    })
}

fn console_endpoint(
    endpoint: crate::grpc::pb::nvbes::developer::v1::WebhookEndpoint,
) -> Result<DeveloperWebhookEndpointSummary, AppError> {
    Ok(DeveloperWebhookEndpointSummary {
        id: uuid(&endpoint.endpoint_id, "endpoint id")?,
        name: endpoint.name,
        url: endpoint.url,
        status: endpoint.status,
        failed_delivery_count: endpoint.failed_delivery_count,
        created_at: time(&endpoint.created_at, "created_at")?,
        updated_at: time(&endpoint.updated_at, "updated_at")?,
    })
}

fn delivery(
    delivery: crate::grpc::pb::nvbes::developer::v1::WebhookDelivery,
) -> Result<DeveloperWebhookDeliverySummary, AppError> {
    Ok(DeveloperWebhookDeliverySummary {
        id: uuid(&delivery.delivery_id, "delivery id")?,
        endpoint_id: uuid(&delivery.endpoint_id, "endpoint id")?,
        event_id: uuid(&delivery.event_id, "event id")?,
        event_type: delivery.event_type,
        status: delivery.status,
        attempt_count: delivery.attempt_count,
        response_status: if delivery.response_status.is_empty() {
            None
        } else {
            delivery.response_status.parse().ok()
        },
        error_message: optional_string(delivery.error_message),
        created_at: time(&delivery.created_at, "created_at")?,
        delivered_at: optional_time(&delivery.delivered_at, "delivered_at")?,
        replayed_from_delivery_id: if delivery.replayed_from_delivery_id.is_empty() {
            None
        } else {
            Some(uuid(
                &delivery.replayed_from_delivery_id,
                "replayed delivery id",
            )?)
        },
    })
}

fn parse_event_type(value: String) -> Result<DeveloperWebhookEventType, AppError> {
    match value.as_str() {
        "user.created" => Ok(DeveloperWebhookEventType::UserCreated),
        "login.failed" => Ok(DeveloperWebhookEventType::LoginFailed),
        "session.revoked" => Ok(DeveloperWebhookEventType::SessionRevoked),
        "client.created" => Ok(DeveloperWebhookEventType::ClientCreated),
        _ => Err(AppError::internal(
            "developer_contract_invalid",
            format!("Unknown Developer webhook event type: {value}."),
        )),
    }
}
