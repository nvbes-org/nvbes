pub use nvbes_product_account::email::{db, jobs};
#[path = "identity.email.routes.rs"]
pub mod routes;
#[path = "identity.email.templates.rs"]
pub mod templates;
#[path = "identity.email.webhooks.rs"]
pub mod webhooks;

pub use templates::{invitation_email, password_reset_email, verification_email};
