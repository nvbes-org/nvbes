#[path = "identity.domains.authz.db.rs"]
pub mod db;
#[path = "identity.domains.authz.routes.rs"]
pub mod routes;
#[path = "identity.domains.authz.service.rs"]
pub mod service;
#[path = "identity.domains.authz.types.rs"]
pub mod types;

pub use db::target_role_for_member;
pub use service::*;
pub use types::*;
