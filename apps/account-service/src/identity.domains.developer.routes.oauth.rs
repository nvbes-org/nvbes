#[path = "identity.domains.developer.routes.oauth.clients.rs"]
mod clients;
#[path = "identity.domains.developer.routes.oauth.consent.rs"]
mod consent;
#[path = "identity.domains.developer.routes.oauth.marketplace.rs"]
mod marketplace;
#[path = "identity.domains.developer.routes.oauth.scopes.rs"]
mod scopes;

pub use clients::list_oauth_clients;
pub use consent::{get_consent_screen, upsert_consent_screen};
pub use marketplace::{list_marketplace_apps, review_marketplace_app, submit_marketplace_app};
pub use scopes::{create_scope, delete_scope, list_scopes, update_scope};
