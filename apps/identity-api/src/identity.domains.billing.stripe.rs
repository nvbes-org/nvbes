#[path = "identity.domains.billing.stripe.customer.rs"]
mod customer;
#[path = "identity.domains.billing.stripe.http.rs"]
mod http;
#[path = "identity.domains.billing.stripe.sessions.rs"]
mod sessions;
#[cfg(test)]
#[path = "identity.domains.billing.stripe.tests.rs"]
mod tests;
#[path = "identity.domains.billing.stripe.webhooks.rs"]
mod webhooks;

pub use customer::{build_customer_fields, create_stripe_customer};
pub use http::stripe_post_form;
pub use sessions::{
    build_checkout_session_fields, create_stripe_checkout_session, create_stripe_portal_session,
};
pub use webhooks::parse_stripe_event;
