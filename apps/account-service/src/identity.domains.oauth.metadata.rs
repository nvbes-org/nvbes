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
    pub authorization_details_types_supported: Vec<&'static str>,
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
        token_endpoint_auth_signing_alg_values_supported: vec!["RS256", "RS384", "RS512", "ES256"],
        subject_types_supported: vec!["public", "pairwise"],
        id_token_signing_alg_values_supported: vec!["RS256"],
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
            "updated_at",
        ],
        code_challenge_methods_supported: vec!["S256"],
        dpop_signing_alg_values_supported: vec!["ES256"],
        scopes_supported: vec![
            "openid",
            "profile",
            "email",
            "offline_access",
            "drive:read",
            "drive:write",
            "drive:admin",
        ],
        request_parameter_supported: true,
        request_uri_parameter_supported: true,
        require_pushed_authorization_requests: true,
        authorization_response_iss_parameter_supported: true,
        authorization_details_types_supported: vec!["drive:file", "drive:workspace"],
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

#[cfg(test)]
mod tests {
    use super::endpoint;

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
}
