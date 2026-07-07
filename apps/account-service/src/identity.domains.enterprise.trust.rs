#[path = "identity.domains.enterprise.trust.service.rs"]
mod service;
#[path = "identity.domains.enterprise.trust.types.rs"]
pub mod types;

pub use service::get_trust_center;
