#[cfg(test)]
#[path = "identity.domains.oauth.contract.tests.rs"]
mod contract_tests;
#[path = "identity.domains.oauth.device.rs"]
pub mod device;
#[path = "identity.domains.oauth.device.authorization.rs"]
pub mod device_authorization;
#[path = "identity.domains.oauth.device.exchange.rs"]
pub mod device_exchange;
#[path = "identity.domains.oauth.device.validation.rs"]
pub mod device_validation;
#[path = "identity.domains.oauth.device.verification.rs"]
pub mod device_verification;
#[cfg(test)]
#[path = "identity.domains.oauth.hosted.tests.rs"]
mod hosted_tests;
#[path = "identity.domains.oauth.logic.rs"]
pub mod logic;
#[path = "identity.domains.oauth.metadata.rs"]
pub mod metadata;
#[path = "identity.domains.oauth.routes.rs"]
pub mod routes;
#[path = "identity.domains.oauth.service.rs"]
pub mod service;
#[path = "identity.domains.oauth.validation.rs"]
pub mod validation;

#[path = "identity.domains.oauth.assurance.rs"]
pub mod assurance;
#[path = "identity.domains.oauth.authorization_codes.rs"]
pub mod authorization_codes;
#[path = "identity.domains.oauth.client_assertion.rs"]
pub mod client_assertion;
#[path = "identity.domains.oauth.clients.rs"]
pub mod clients;
#[path = "identity.domains.oauth.consent.rs"]
pub mod consent;
#[path = "identity.domains.oauth.device_codes.rs"]
pub mod device_codes;
#[path = "identity.domains.oauth.flows.rs"]
pub mod flows;
#[path = "identity.domains.oauth.hosted.keys.rs"]
pub mod hosted_keys;
#[path = "identity.domains.oauth.hosted.routes.rs"]
pub mod hosted_routes;
#[path = "identity.domains.oauth.hosted.service.rs"]
pub mod hosted_service;
#[path = "identity.domains.oauth.hosted.store.rs"]
pub mod hosted_store;
#[path = "identity.domains.oauth.hosted.types.rs"]
pub mod hosted_types;
#[path = "identity.domains.oauth.jar.rs"]
pub mod jar;
#[path = "identity.domains.oauth.policies.rs"]
pub mod policies;
#[path = "identity.domains.oauth.policies.eval.rs"]
pub mod policies_eval;
#[path = "identity.domains.oauth.rar.rs"]
pub mod rar;

pub use logic::{generate_user_code, hash_client_secret, verify_client_secret};
pub use validation::validate_pkce_for_exchange as verify_pkce;
pub use validation::{
    normalize_resources, normalize_scopes, parse_client_policy_status, parse_client_type,
    parse_step_up_level,
};
