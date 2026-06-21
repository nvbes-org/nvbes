#[path = "identity.email.webhooks.handlers.rs"]
pub mod handlers;

pub use handlers::{EmailProviderEvent, EmailWebhookResponse, webhook_router};
