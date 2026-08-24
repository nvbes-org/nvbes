#[path = "trust_risk.assessment.rs"]
pub mod assessment;
#[path = "trust_risk.attribute.rs"]
pub mod attribute;
#[path = "trust_risk.client.rs"]
pub mod client;
#[path = "trust_risk.label.rs"]
pub mod label;
pub mod proto;
#[path = "trust_risk.rules.rs"]
pub mod rules;
#[path = "trust_risk.signal.rs"]
pub mod signal;
#[path = "trust_risk.types.rs"]
pub mod types;

#[cfg(test)]
#[path = "trust_risk.contract.tests.rs"]
mod contract_tests;
#[cfg(test)]
#[path = "trust_risk.domain.tests.rs"]
mod domain_tests;
