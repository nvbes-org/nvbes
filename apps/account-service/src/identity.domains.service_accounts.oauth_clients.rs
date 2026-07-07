#[path = "identity.domains.service_accounts.oauth_clients.attach.rs"]
mod attach;
#[path = "identity.domains.service_accounts.oauth_clients.create.rs"]
mod create;
#[path = "identity.domains.service_accounts.oauth_clients.revoke.rs"]
mod revoke;
#[path = "identity.domains.service_accounts.oauth_clients.rotate.rs"]
mod rotate;

pub use attach::attach_oauth_client;
pub use create::create_service_account_oauth_client;
pub use revoke::revoke_oauth_client;
pub use rotate::rotate_oauth_client_secret;
