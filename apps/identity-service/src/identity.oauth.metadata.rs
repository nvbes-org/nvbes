use serde::Serialize;

/// OIDC discovery metadata is deliberately derived from one configured origin.
/// Endpoints that are not mounted are not advertised here.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ProviderMetadata {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub userinfo_endpoint: String,
    pub jwks_uri: String,
    pub response_types_supported: Vec<&'static str>,
    pub response_modes_supported: Vec<&'static str>,
    pub grant_types_supported: Vec<&'static str>,
    pub code_challenge_methods_supported: Vec<&'static str>,
    pub subject_types_supported: Vec<&'static str>,
    pub id_token_signing_alg_values_supported: Vec<&'static str>,
    pub scopes_supported: Vec<&'static str>,
}

pub fn provider_metadata(issuer: &str) -> ProviderMetadata {
    ProviderMetadata {
        issuer: issuer.into(),
        authorization_endpoint: format!("{issuer}oauth/authorize"),
        token_endpoint: format!("{issuer}oauth/token"),
        userinfo_endpoint: format!("{issuer}oauth/userinfo"),
        jwks_uri: format!("{issuer}oauth/jwks"),
        response_types_supported: vec!["code"],
        response_modes_supported: vec!["query"],
        grant_types_supported: vec!["authorization_code"],
        code_challenge_methods_supported: vec!["S256"],
        subject_types_supported: vec!["public"],
        id_token_signing_alg_values_supported: vec!["RS256"],
        scopes_supported: vec!["openid", "profile", "email", "offline_access"],
    }
}

#[cfg(test)]
#[path = "identity.oauth.metadata.tests.rs"]
mod tests;
