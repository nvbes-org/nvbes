#[path = "identity.domains.federation.contract.rs"]
pub mod contract;
#[path = "identity.domains.federation.domains.rs"]
pub mod domains;
#[path = "identity.domains.federation.http.rs"]
pub(crate) mod http;
#[path = "identity.domains.federation.identities.rs"]
pub mod identities;
#[path = "identity.domains.federation.oidc.rs"]
pub mod oidc;
#[path = "identity.domains.federation.providers.rs"]
pub mod providers;
#[path = "identity.domains.federation.provisioning.rs"]
pub mod provisioning;
#[path = "identity.domains.federation.routes.rs"]
pub mod routes;
#[path = "identity.domains.federation.saml.rs"]
pub mod saml;
#[path = "identity.domains.federation.saml.authn.rs"]
pub mod saml_authn;
#[path = "identity.domains.federation.saml.replay.rs"]
pub mod saml_replay;
#[path = "identity.domains.federation.saml.subject.rs"]
pub mod saml_subject;
#[path = "identity.domains.federation.saml.validation.rs"]
pub mod saml_validation;
#[path = "identity.domains.federation.scim.rs"]
pub mod scim;
#[path = "identity.domains.federation.sso_policy.rs"]
pub mod sso_policy;
#[path = "identity.domains.federation.types.rs"]
pub mod types;
#[path = "identity.domains.federation.validation.rs"]
pub mod validation;

#[cfg(test)]
#[path = "identity.domains.federation.contract.tests.rs"]
mod contract_tests;
