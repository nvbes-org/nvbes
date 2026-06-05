#[path = "identity.domains.auth.password.enterprise.approval.rs"]
pub mod approval;
#[path = "identity.domains.auth.password.enterprise.rules.rs"]
pub mod rules;

pub use approval::approve_enterprise_recovery;
