use serde::{Deserialize, Serialize};

use crate::app::AppState;

#[derive(Debug, Serialize)]
pub struct EmailWebhookResponse {
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct EmailProviderEvent {
    pub id: String,
    pub email_id: String,
    pub event_type: String,
    pub email: String,
    pub timestamp: String,
    #[serde(default)]
    pub message: Option<EmailProviderMessage>,
}

#[derive(Debug, Deserialize)]
pub struct EmailProviderMessage {
    pub subject: Option<String>,
    pub message_id: Option<String>,
}

pub fn webhook_router(_state: &AppState) -> axum::Router<AppState> {
    axum::Router::new()
}

pub fn is_actionable_event(event_type: &str) -> bool {
    matches!(
        event_type,
        "email_mailbox_not_found"
            | "email_spam"
            | "email_bounced"
            | "email_blacklisted"
            | "email_unsubscribed"
            | "email_delivered"
            | "email_dropped"
    )
}
