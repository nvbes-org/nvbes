#[path = "identity.domains.developer.rbac.rs"]
pub mod rbac;
#[path = "identity.domains.developer.rbac.db.rs"]
pub mod rbac_db;
#[path = "identity.domains.developer.routes.rs"]
pub mod routes;
#[path = "identity.domains.developer.service.rs"]
pub mod service;
#[path = "identity.domains.developer.service_accounts.db.rs"]
pub mod service_accounts_db;
#[path = "identity.domains.developer.types.rs"]
pub mod types;

#[cfg(test)]
#[path = "identity.domains.developer.tests.rs"]
mod tests;
