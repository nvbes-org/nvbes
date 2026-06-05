#[path = "identity.email.webhooks.handlers.rs"]
pub mod handlers;

pub use handlers::{EmailWebhookResponse, ScalewayEmailEvent, webhook_router};
