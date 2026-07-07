use super::authenticate_tenant;
use crate::app::AppState;
use crate::domains::federation::types::{
    CreateFederatedIdentityProviderInput, FederatedIdentityProvidersResponse,
    OidcDiscoveryResponse, SamlMetadataResponse, UpdateFederatedIdentityProviderInput,
};
use crate::http::error::AppError;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, patch},
};
use nvbes_core::http::error::ErrorEnvelope;
use serde::Deserialize;
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/tenants/{tenantId}/identity-providers",
            get(list_identity_providers).post(create_identity_provider),
        )
        .route(
            "/tenants/{tenantId}/identity-providers/{providerId}",
            patch(update_identity_provider).delete(delete_identity_provider),
        )
        .route(
            "/tenants/{tenantId}/identity-providers/{providerId}/oidc/discovery",
            get(oidc_discovery),
        )
        .route(
            "/tenants/{tenantId}/identity-providers/{providerId}/saml/metadata",
            get(saml_metadata),
        )
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) struct CreateFederatedIdentityProviderRequest {
    provider_type: String,
    provider_family: Option<String>,
    name: String,
    client_id: Option<String>,
    issuer: Option<String>,
    metadata_url: Option<String>,
    status: Option<String>,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) struct UpdateFederatedIdentityProviderRequest {
    provider_type: Option<String>,
    provider_family: Option<String>,
    name: Option<String>,
    client_id: Option<String>,
    issuer: Option<String>,
    metadata_url: Option<String>,
    status: Option<String>,
}

#[utoipa::path(
    get,
    path = "/tenants/{tenantId}/identity-providers",
    tag = "federation",
    params(
        ("tenantId" = Uuid, Path, description = "Tenant ID"),
    ),
    responses(
        (status = 200, description = "List identity providers", body = FederatedIdentityProvidersResponse),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn list_identity_providers(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(tenant_id): Path<Uuid>,
) -> Result<Json<FederatedIdentityProvidersResponse>, AppError> {
    let auth = authenticate_tenant(&state, &headers, tenant_id).await?;
    Ok(Json(
        crate::domains::federation::providers::list_identity_providers(
            &state.db,
            tenant_id,
            auth.user_id,
        )
        .await?,
    ))
}

#[utoipa::path(
    post,
    path = "/tenants/{tenantId}/identity-providers",
    tag = "federation",
    params(
        ("tenantId" = Uuid, Path, description = "Tenant ID"),
    ),
    request_body = CreateFederatedIdentityProviderRequest,
    responses(
        (status = 200, description = "Identity provider created", body = crate::domains::federation::types::FederatedIdentityProviderResponse),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn create_identity_provider(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(tenant_id): Path<Uuid>,
    Json(request): Json<CreateFederatedIdentityProviderRequest>,
) -> Result<Json<crate::domains::federation::types::FederatedIdentityProviderResponse>, AppError> {
    let auth = authenticate_tenant(&state, &headers, tenant_id).await?;
    Ok(Json(
        crate::domains::federation::providers::create_identity_provider(
            &state.db,
            tenant_id,
            auth.user_id,
            CreateFederatedIdentityProviderInput {
                provider_type: request.provider_type,
                provider_family: request.provider_family,
                name: request.name,
                client_id: request.client_id,
                issuer: request.issuer,
                metadata_url: request.metadata_url,
                status: request.status,
            },
            state.config.environment != "development",
        )
        .await?,
    ))
}

#[utoipa::path(
    patch,
    path = "/tenants/{tenantId}/identity-providers/{providerId}",
    tag = "federation",
    params(
        ("tenantId" = Uuid, Path, description = "Tenant ID"),
        ("providerId" = Uuid, Path, description = "Provider ID"),
    ),
    request_body = UpdateFederatedIdentityProviderRequest,
    responses(
        (status = 200, description = "Identity provider updated", body = crate::domains::federation::types::FederatedIdentityProviderResponse),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 404, description = "Not found", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn update_identity_provider(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((tenant_id, provider_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<UpdateFederatedIdentityProviderRequest>,
) -> Result<Json<crate::domains::federation::types::FederatedIdentityProviderResponse>, AppError> {
    let auth = authenticate_tenant(&state, &headers, tenant_id).await?;
    Ok(Json(
        crate::domains::federation::providers::update_identity_provider(
            &state.db,
            tenant_id,
            auth.user_id,
            provider_id,
            UpdateFederatedIdentityProviderInput {
                provider_type: request.provider_type,
                provider_family: request.provider_family,
                name: request.name,
                client_id: request.client_id,
                issuer: request.issuer,
                metadata_url: request.metadata_url,
                status: request.status,
            },
            state.config.environment != "development",
        )
        .await?,
    ))
}

#[utoipa::path(
    delete,
    path = "/tenants/{tenantId}/identity-providers/{providerId}",
    tag = "federation",
    params(
        ("tenantId" = Uuid, Path, description = "Tenant ID"),
        ("providerId" = Uuid, Path, description = "Provider ID"),
    ),
    responses(
        (status = 200, description = "Identity provider deleted"),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 404, description = "Not found", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn delete_identity_provider(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((tenant_id, provider_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let auth = authenticate_tenant(&state, &headers, tenant_id).await?;
    crate::domains::federation::providers::delete_identity_provider(
        &state.db,
        tenant_id,
        auth.user_id,
        provider_id,
    )
    .await?;
    Ok(Json(serde_json::json!({ "success": true })))
}

#[utoipa::path(
    get,
    path = "/tenants/{tenantId}/identity-providers/{providerId}/oidc/discovery",
    tag = "federation",
    params(
        ("tenantId" = Uuid, Path, description = "Tenant ID"),
        ("providerId" = Uuid, Path, description = "Provider ID"),
    ),
    responses(
        (status = 200, description = "OIDC discovery document", body = OidcDiscoveryResponse),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 404, description = "Not found", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn oidc_discovery(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((tenant_id, provider_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<OidcDiscoveryResponse>, AppError> {
    let _auth = authenticate_tenant(&state, &headers, tenant_id).await?;
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
    let discovery =
        crate::domains::federation::oidc::fetch_oidc_discovery(&provider, strict_mode).await?;
    Ok(Json(OidcDiscoveryResponse {
        discovery_url: crate::domains::federation::oidc::oidc_discovery_url(&provider),
        issuer: discovery.issuer,
        authorization_endpoint: discovery.authorization_endpoint,
        token_endpoint: discovery.token_endpoint,
        userinfo_endpoint: discovery.userinfo_endpoint,
        jwks_uri: discovery.jwks_uri,
        response_types_supported: discovery.response_types_supported,
        subject_types_supported: discovery.subject_types_supported,
        id_token_signing_alg_values_supported: discovery.id_token_signing_alg_values_supported,
        claims_supported: discovery.claims_supported,
        scopes_supported: discovery.scopes_supported,
    }))
}

#[utoipa::path(
    get,
    path = "/tenants/{tenantId}/identity-providers/{providerId}/saml/metadata",
    tag = "federation",
    params(
        ("tenantId" = Uuid, Path, description = "Tenant ID"),
        ("providerId" = Uuid, Path, description = "Provider ID"),
    ),
    responses(
        (status = 200, description = "SAML metadata", body = SamlMetadataResponse),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 404, description = "Not found", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn saml_metadata(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((tenant_id, provider_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<SamlMetadataResponse>, AppError> {
    let _auth = authenticate_tenant(&state, &headers, tenant_id).await?;
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
    let metadata =
        crate::domains::federation::saml::fetch_saml_metadata(&provider, strict_mode).await?;
    Ok(Json(metadata))
}
