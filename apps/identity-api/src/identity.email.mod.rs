#[path = "identity.email.db.rs"]
pub mod db;
#[path = "identity.email.jobs.rs"]
pub mod jobs;
#[path = "identity.email.routes.rs"]
pub mod routes;
#[path = "identity.email.templates.rs"]
pub mod templates;
#[path = "identity.email.webhooks.rs"]
pub mod webhooks;

pub use templates::{invitation_email, password_reset_email, verification_email};
