use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// Event types supported by Developer webhooks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub enum DeveloperWebhookEventType {
    #[serde(rename = "user.created")]
    UserCreated,
    #[serde(rename = "login.failed")]
    LoginFailed,
    #[serde(rename = "session.revoked")]
    SessionRevoked,
    #[serde(rename = "client.created")]
    ClientCreated,
}

impl DeveloperWebhookEventType {
    pub fn as_event_type(self) -> &'static str {
        match self {
            DeveloperWebhookEventType::UserCreated => "user.created",
            DeveloperWebhookEventType::LoginFailed => "login.failed",
            DeveloperWebhookEventType::SessionRevoked => "session.revoked",
            DeveloperWebhookEventType::ClientCreated => "client.created",
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DeveloperWebhookEndpointView {
    pub id: Uuid,
    pub name: String,
    pub url: String,
    pub status: String,
    pub events: Vec<DeveloperWebhookEventType>,
    pub signing_secret_last4: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateDeveloperWebhookEndpointRequest {
    pub name: String,
    pub url: String,
    pub events: Vec<DeveloperWebhookEventType>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreateDeveloperWebhookEndpointResponse {
    pub endpoint: DeveloperWebhookEndpointView,
    pub signing_secret: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DeveloperWebhooksResponse {
    pub endpoints: Vec<DeveloperWebhookEndpointView>,
}

#[derive(Debug, FromRow, Serialize)]
pub struct DeveloperWebhookEndpointSummary {
    pub id: Uuid,
    pub name: String,
    pub url: String,
    pub status: String,
    pub failed_delivery_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct DeveloperWebhookEndpointsResponse {
    pub webhooks: Vec<DeveloperWebhookEndpointSummary>,
}

#[derive(Debug, FromRow, Serialize)]
pub struct DeveloperWebhookDeliverySummary {
    pub id: Uuid,
    pub endpoint_id: Uuid,
    pub event_id: Uuid,
    pub event_type: String,
    pub status: String,
    pub attempt_count: i32,
    pub response_status: Option<i32>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub delivered_at: Option<DateTime<Utc>>,
    pub replayed_from_delivery_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct DeveloperWebhookDeliveriesResponse {
    pub deliveries: Vec<DeveloperWebhookDeliverySummary>,
}
