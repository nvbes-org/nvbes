#[path = "identity.domains.billing.webhooks.handlers.rs"]
pub mod handlers;
#[path = "identity.domains.billing.webhooks.logic.rs"]
pub mod logic;
#[path = "identity.domains.billing.webhooks.processors.rs"]
pub mod processors;
#[cfg(test)]
#[path = "identity.domains.billing.webhooks.tests.checkout.rs"]
mod tests_checkout;
#[cfg(test)]
#[path = "identity.domains.billing.webhooks.tests.invoice.rs"]
mod tests_invoice;
#[cfg(test)]
#[path = "identity.domains.billing.webhooks.tests.logic.rs"]
mod tests_logic;
#[cfg(test)]
#[path = "identity.domains.billing.webhooks.tests.subscription.rs"]
mod tests_subscription;
#[path = "identity.domains.billing.webhooks.validators.checkout.rs"]
pub mod validators_checkout;
#[path = "identity.domains.billing.webhooks.validators.invoice.rs"]
pub mod validators_invoice;
#[path = "identity.domains.billing.webhooks.validators.subscription.rs"]
pub mod validators_subscription;
#[path = "identity.domains.billing.webhooks.validators.workspace.rs"]
pub mod validators_workspace;

pub use handlers::handle_webhook;
pub use processors::process_stripe_event;
