use serde::Deserialize;

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
