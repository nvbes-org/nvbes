#[path = "identity.domains.enterprise.access_reviews.rs"]
pub mod access_reviews;
#[path = "identity.domains.enterprise.admin_elevation.rs"]
pub mod admin_elevation;
#[path = "identity.domains.enterprise.db.rs"]
pub mod db;
#[path = "identity.domains.enterprise.grpc.rs"]
pub mod grpc;
#[path = "identity.domains.enterprise.policy.rs"]
pub mod policy;
#[path = "identity.domains.enterprise.policy_simulation.rs"]
pub mod policy_simulation;
#[path = "identity.domains.enterprise.routes.rs"]
pub mod routes;
#[path = "identity.domains.enterprise.security_posture.rs"]
pub mod security_posture;
#[path = "identity.domains.enterprise.service.rs"]
pub mod service;
#[path = "identity.domains.enterprise.trust.rs"]
pub mod trust;
#[path = "identity.domains.enterprise.types.rs"]
pub mod types;

#[cfg(test)]
#[path = "identity.domains.enterprise.tests.rs"]
mod tests;
