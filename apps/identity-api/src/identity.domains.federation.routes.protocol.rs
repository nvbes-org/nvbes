use crate::app::AppState;
use crate::domains::federation::types::{InboundFederationInput, InboundFederationResponse};
use crate::http::error::AppError;
use axum::{
    Form, Json, Router,
    extract::{Path, State},
    routing::post,
};
use nvbes_core::http::error::ErrorEnvelope;
use serde::Deserialize;
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/tenants/{tenantId}/identity-providers/{providerId}/oidc/callback",
            post(oidc_callback),
        )
        .route(
            "/tenants/{tenantId}/identity-providers/{providerId}/saml/acs",
            post(saml_acs),
        )
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) struct OidcCallbackRequest {
    id_token: String,
    nonce: Option<String>,
    #[allow(dead_code)]
    state: Option<String>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub(crate) struct SamlAcsRequest {
    #[serde(rename = "SAMLResponse")]
    saml_response: String,
    #[serde(rename = "RelayState")]
    relay_state: Option<String>,
}

#[utoipa::path(
    post,
    path = "/tenants/{tenantId}/identity-providers/{providerId}/oidc/callback",
    tag = "federation",
    params(
        ("tenantId" = Uuid, Path, description = "Tenant ID"),
        ("providerId" = Uuid, Path, description = "Provider ID"),
    ),
    request_body = OidcCallbackRequest,
    responses(
        (status = 200, description = "OIDC callback processed", body = InboundFederationResponse),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn oidc_callback(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path((tenant_id, provider_id)): Path<(Uuid, Uuid)>,
    Form(request): Form<OidcCallbackRequest>,
) -> Result<Json<InboundFederationResponse>, AppError> {
    super::enforce_federation_public_rate_limit_db(
        &state.redis,
        &headers,
        "federation_oidc_callback",
        &format!("tenant:{tenant_id}:provider:{provider_id}"),
    )
    .await?;

    let provider = crate::domains::federation::providers::fetch_identity_provider(
        &state.db,
        tenant_id,
        provider_id,
    )
    .await?;
    if provider.provider_type != "oidc" {
        return Err(AppError::bad_request(
            crate::domains::federation::contract::PROVIDER_TYPE_MISMATCH,
            "The identity provider is not configured for OIDC.",
        ));
    }
    let strict_mode = state.config.environment != "development";
    let oidc = crate::domains::federation::oidc::validate_oidc_id_token(
        &provider,
        &request.id_token,
        request.nonce.as_deref(),
        strict_mode,
    )
    .await?;
    let email = oidc.email.clone().ok_or_else(|| {
        AppError::bad_request(
            crate::domains::federation::contract::MISSING_EMAIL,
            "The ID token does not contain an email address.",
        )
    })?;
    let username = oidc.username();
    let email_verified = oidc.email_verified.unwrap_or(true);
    let input = InboundFederationInput {
        provider_id,
        email,
        username,
        subject: oidc.sub,
        email_verified,
    };

    Ok(Json(
        crate::domains::federation::provisioning::handle_inbound_federation(
            &state.db,
            &state.redis,
            &state.jwt,
            state.config.auth_session_ttl_hours,
            tenant_id,
            provider,
            input,
            "oidc",
        )
        .await?,
    ))
}

#[utoipa::path(
    post,
    path = "/tenants/{tenantId}/identity-providers/{providerId}/saml/acs",
    tag = "federation",
    params(
        ("tenantId" = Uuid, Path, description = "Tenant ID"),
        ("providerId" = Uuid, Path, description = "Provider ID"),
    ),
    request_body = SamlAcsRequest,
    responses(
        (status = 200, description = "SAML ACS processed", body = InboundFederationResponse),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn saml_acs(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path((tenant_id, provider_id)): Path<(Uuid, Uuid)>,
    Form(request): Form<SamlAcsRequest>,
) -> Result<Json<InboundFederationResponse>, AppError> {
    super::enforce_federation_public_rate_limit_db(
        &state.redis,
        &headers,
        "federation_saml_acs",
        &format!("tenant:{tenant_id}:provider:{provider_id}"),
    )
    .await?;

    let provider = crate::domains::federation::providers::fetch_identity_provider(
        &state.db,
        tenant_id,
        provider_id,
    )
    .await?;
    if provider.provider_type != "saml" {
        return Err(AppError::bad_request(
            crate::domains::federation::contract::PROVIDER_TYPE_MISMATCH,
            "The identity provider is not configured for SAML.",
        ));
    }
    let strict_mode = state.config.environment != "development";
    let expected_recipient = saml_acs_url(&state.config.api_base_url, tenant_id, provider_id);
    let saml_assertion = crate::domains::federation::saml::validate_saml_response(
        &state.db,
        tenant_id,
        provider_id,
        &provider,
        &request.saml_response,
        request.relay_state.as_deref(),
        strict_mode,
        &expected_recipient,
    )
    .await?;
    let email = saml_assertion.email.clone().ok_or_else(|| {
        AppError::bad_request(
            crate::domains::federation::contract::MISSING_EMAIL,
            "The SAML assertion does not contain an email address.",
        )
    })?;
    let input = InboundFederationInput {
        provider_id,
        email,
        username: saml_assertion.username(),
        subject: saml_assertion.name_id.clone(),
        email_verified: true,
    };

    Ok(Json(
        crate::domains::federation::provisioning::handle_inbound_federation(
            &state.db,
            &state.redis,
            &state.jwt,
            state.config.auth_session_ttl_hours,
            tenant_id,
            provider,
            input,
            "saml",
        )
        .await?,
    ))
}

fn saml_acs_url(api_base_url: &str, tenant_id: Uuid, provider_id: Uuid) -> String {
    format!(
        "{}/tenants/{}/identity-providers/{}/saml/acs",
        api_base_url.trim_end_matches('/'),
        tenant_id,
        provider_id
    )
}

#[cfg(test)]
mod tests {
    use super::saml_acs_url;
    use uuid::Uuid;

    #[test]
    fn saml_acs_url_uses_public_api_base_url_and_api_prefix() {
        let tenant_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        let provider_id = Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap();

        let url = saml_acs_url("https://identity.example.com/", tenant_id, provider_id);

        assert_eq!(
            url,
            "https://identity.example.com/tenants/00000000-0000-0000-0000-000000000001/identity-providers/00000000-0000-0000-0000-000000000002/saml/acs"
        );
    }
}
