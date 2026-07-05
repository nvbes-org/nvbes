#[path = "drive.domains.public_api.api_key_signatures.rs"]
pub mod api_key_signatures;
#[path = "drive.domains.public_api.auth.rs"]
pub mod auth;
#[path = "drive.domains.public_api.db.rs"]
pub mod db;
#[cfg(test)]
#[path = "drive.domains.public_api.db_tests.rs"]
mod db_tests;
#[cfg(test)]
#[path = "drive.domains.public_api.e2e_tests.rs"]
mod e2e_tests;
#[path = "drive.domains.public_api.errors.rs"]
pub mod errors;
#[path = "drive.domains.public_api.geo.rs"]
pub mod geo;
#[path = "drive.domains.public_api.http_signatures.rs"]
pub mod http_signatures;
#[path = "drive.domains.public_api.metrics.rs"]
pub mod metrics;
#[path = "drive.domains.public_api.routes.mgmt_handlers.rs"]
pub mod mgmt_handlers;
#[path = "drive.domains.public_api.network_policy.rs"]
pub mod network_policy;
#[path = "drive.domains.public_api.observability.rs"]
pub mod observability;
#[path = "drive.domains.public_api.request_meta.rs"]
pub mod request_meta;
#[path = "drive.domains.public_api.routes.rs"]
pub mod routes;
#[path = "drive.domains.public_api.routes.access.rs"]
pub mod routes_access;
#[path = "drive.domains.public_api.routes.audit.rs"]
pub mod routes_audit;
#[path = "drive.domains.public_api.service.rs"]
pub mod service;
#[path = "drive.domains.public_api.types.rs"]
pub mod types;
#[path = "drive.domains.public_api.routes.v1_handlers.mod.rs"]
pub mod v1_handlers;

pub use routes::router;
pub use service::{
    authenticate, authorize_access, list_api_keys, list_workspaces, log_request, me,
    record_api_audit_event, revoke_api_key,
};
