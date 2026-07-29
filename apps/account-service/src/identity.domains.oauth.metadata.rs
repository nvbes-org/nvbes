use axum::{Json, extract::State};
use nvbes_core::http::error::ErrorEnvelope;
use serde::Serialize;

use crate::{app::AppState, http::error::AppError};

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct OAuthAuthorizationServerMetadata {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub jwks_uri: String,
    pub userinfo_endpoint: String,
    pub introspection_endpoint: String,
    pub revocation_endpoint: String,
    pub pushed_authorization_request_endpoint: String,
    pub device_authorization_endpoint: String,
    pub response_types_supported: Vec<&'static str>,
    pub grant_types_supported: Vec<&'static str>,
    pub token_endpoint_auth_methods_supported: Vec<&'static str>,
    pub token_endpoint_auth_signing_alg_values_supported: Vec<&'static str>,
    pub request_object_signing_alg_values_supported: Vec<&'static str>,
    pub subject_types_supported: Vec<&'static str>,
    pub id_token_signing_alg_values_supported: Vec<&'static str>,
    pub claims_supported: Vec<&'static str>,
    pub code_challenge_methods_supported: Vec<&'static str>,
    pub dpop_signing_alg_values_supported: Vec<&'static str>,
    pub scopes_supported: Vec<&'static str>,
    pub request_parameter_supported: bool,
    pub request_uri_parameter_supported: bool,
    pub require_pushed_authorization_requests: bool,
    pub authorization_response_iss_parameter_supported: bool,
    pub backchannel_logout_supported: bool,
    pub backchannel_logout_session_supported: bool,
    pub authorization_details_types_supported: Vec<&'static str>,
    pub tls_client_certificate_bound_access_tokens: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mtls_endpoint_aliases: Option<MtlsEndpointAliases>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct MtlsEndpointAliases {
    pub token_endpoint: String,
    pub pushed_authorization_request_endpoint: String,
    pub introspection_endpoint: String,
    pub revocation_endpoint: String,
}

#[utoipa::path(
    get,
    path = "/.well-known/oauth-authorization-server",
    tag = "oauth",
    responses(
        (status = 200, description = "OAuth authorization server metadata", body = OAuthAuthorizationServerMetadata),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub async fn oauth_authorization_server_metadata(
    State(state): State<AppState>,
) -> Result<Json<OAuthAuthorizationServerMetadata>, AppError> {
    let issuer = state.config.api_base_url.trim_end_matches('/').to_string();

    Ok(Json(OAuthAuthorizationServerMetadata {
        authorization_endpoint: endpoint(&issuer, "/oauth/authorize"),
        token_endpoint: endpoint(&issuer, "/oauth/token"),
        jwks_uri: endpoint(&issuer, "/.well-known/jwks.json"),
        userinfo_endpoint: endpoint(&issuer, "/oauth/userinfo"),
        introspection_endpoint: endpoint(&issuer, "/oauth/introspect"),
        revocation_endpoint: endpoint(&issuer, "/oauth/revoke"),
        pushed_authorization_request_endpoint: endpoint(&issuer, "/oauth/par"),
        device_authorization_endpoint: endpoint(&issuer, "/oauth/device/authorize"),
        issuer,
        response_types_supported: vec!["code"],
        grant_types_supported: vec![
            "authorization_code",
            "refresh_token",
            "client_credentials",
            "urn:ietf:params:oauth:grant-type:device_code",
            "urn:ietf:params:oauth:grant-type:token-exchange",
        ],
        token_endpoint_auth_methods_supported: vec![
            "client_secret_basic",
            "client_secret_post",
            "private_key_jwt",
        ],
        token_endpoint_auth_signing_alg_values_supported: vec![
            "PS256", "ES256", "EdDSA", "RS256", "RS384", "RS512",
        ],
        request_object_signing_alg_values_supported: vec!["PS256", "ES256", "EdDSA"],
        subject_types_supported: vec!["public", "pairwise"],
        id_token_signing_alg_values_supported: vec!["PS256"],
        claims_supported: vec![
            "sub",
            "sid",
            "email",
            "email_verified",
            "nonce",
            "display_name",
            "preferred_username",
            "tenant_id",
            "organization_id",
            "workspace_id",
            "scope",
            "client_id",
            "acr",
            "amr",
            "auth_time",
            "cnf",
            "updated_at",
        ],
        code_challenge_methods_supported: vec!["S256"],
        dpop_signing_alg_values_supported: vec!["ES256"],
        scopes_supported: supported_scopes(),
        request_parameter_supported: true,
        request_uri_parameter_supported: true,
        require_pushed_authorization_requests: true,
        authorization_response_iss_parameter_supported: true,
        backchannel_logout_supported: true,
        backchannel_logout_session_supported: true,
        authorization_details_types_supported: vec!["drive:file", "drive:workspace"],
        tls_client_certificate_bound_access_tokens: state.config.mtls_enabled,
        mtls_endpoint_aliases: state.config.mtls_enabled.then(|| MtlsEndpointAliases {
            token_endpoint: mtls_endpoint(&state, "/oauth/token"),
            pushed_authorization_request_endpoint: mtls_endpoint(&state, "/oauth/par"),
            introspection_endpoint: mtls_endpoint(&state, "/oauth/introspect"),
            revocation_endpoint: mtls_endpoint(&state, "/oauth/revoke"),
        }),
    }))
}

#[utoipa::path(
    get,
    path = "/.well-known/openid-configuration",
    tag = "oauth",
    responses(
        (status = 200, description = "OpenID Connect discovery metadata", body = OAuthAuthorizationServerMetadata),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub async fn openid_configuration(
    State(state): State<AppState>,
) -> Result<Json<OAuthAuthorizationServerMetadata>, AppError> {
    oauth_authorization_server_metadata(State(state)).await
}

fn endpoint(issuer: &str, path: &str) -> String {
    format!("{issuer}{path}")
}

fn mtls_endpoint(state: &AppState, path: &str) -> String {
    let base = url::Url::parse(&state.config.api_base_url)
        .ok()
        .and_then(|mut url| {
            url.set_port(Some(state.config.mtls_port)).ok()?;
            Some(url.to_string())
        })
        .unwrap_or_else(|| state.config.api_base_url.clone());
    endpoint(base.trim_end_matches('/'), path)
}

pub(crate) fn supported_scopes() -> Vec<&'static str> {
    let mut scopes = vec![
        "openid",
        "profile",
        "email",
        "offline_access",
        "drive:read",
        "drive:write",
        "drive:admin",
    ];
    scopes
        .extend_from_slice(crate::http::middleware::jwt::account_access::SUPPORTED_ACCOUNT_SCOPES);
    scopes
}

#[cfg(test)]
mod tests {
    use super::{endpoint, supported_scopes};

    #[test]
    fn endpoint_joins_normalized_issuer_and_path() {
        assert_eq!(
            endpoint("https://identity.example", "/oauth/token"),
            "https://identity.example/oauth/token"
        );
    }

    #[test]
    fn oauth_metadata_advertises_sellable_foundation_endpoints() {
        let issuer = "https://identity.example";

        assert_eq!(
            endpoint(issuer, "/oauth/authorize"),
            "https://identity.example/oauth/authorize"
        );
        assert_eq!(
            endpoint(issuer, "/oauth/token"),
            "https://identity.example/oauth/token"
        );
        assert_eq!(
            endpoint(issuer, "/oauth/userinfo"),
            "https://identity.example/oauth/userinfo"
        );
        assert_eq!(
            endpoint(issuer, "/.well-known/jwks.json"),
            "https://identity.example/.well-known/jwks.json"
        );
    }

    #[test]
    fn oauth_metadata_advertises_account_capability_scopes() {
        let scopes = supported_scopes();

        assert!(scopes.contains(&"account:profile:read"));
        assert!(scopes.contains(&"account:email:write"));
        assert!(scopes.contains(&"account:security:write"));
        assert!(scopes.contains(&"account:delete"));
    }
}
