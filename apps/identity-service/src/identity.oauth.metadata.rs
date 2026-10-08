use serde::Serialize;

/// OIDC discovery metadata is deliberately derived from one configured origin.
/// Endpoints that are not mounted are not advertised here.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ProviderMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_session_endpoint: Option<String>,
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
    let root = issuer.trim_end_matches('/');
    ProviderMetadata {
        end_session_endpoint: None,
        issuer: issuer.into(),
        authorization_endpoint: format!("{root}/oauth/authorize"),
        token_endpoint: format!("{root}/oauth/token"),
        userinfo_endpoint: format!("{root}/oauth/userinfo"),
        jwks_uri: format!("{root}/oauth/jwks"),
        response_types_supported: vec!["code"],
        response_modes_supported: vec!["query"],
        grant_types_supported: vec!["authorization_code", "refresh_token"],
        code_challenge_methods_supported: vec!["S256"],
        subject_types_supported: vec!["public"],
        id_token_signing_alg_values_supported: vec!["RS256"],
        scopes_supported: vec!["openid", "profile", "email", "offline_access"],
    }
}

#[cfg(test)]
#[path = "identity.oauth.metadata.tests.rs"]
mod tests;
