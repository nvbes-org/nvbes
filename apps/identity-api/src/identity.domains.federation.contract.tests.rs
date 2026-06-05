use std::collections::HashSet;

use crate::domains::federation::contract::{
    CONNECTOR_NOT_FOUND, DOMAIN_NOT_FOUND, DOMAIN_NOT_VERIFIED, EMAIL_NOT_VERIFIED,
    IDENTITY_CONFLICT, INVALID_ID_TOKEN, INVALID_PROVIDER_ID, INVALID_SAML_RESPONSE, JWKS_MISSING,
    MISSING_EMAIL, OIDC_DISCOVERY_FETCH_FAILED, OIDC_DISCOVERY_INVALID, PRINCIPAL_CONFLICT,
    PROVIDER_INACTIVE, PROVIDER_NOT_FOUND, PROVIDER_TYPE_MISMATCH, SAML_METADATA_FETCH_FAILED,
    SAML_METADATA_INVALID, SAML_SIGNATURE_INVALID, TENANT_CONTEXT_REQUIRED, TENANT_MISMATCH,
};
use crate::http::openapi::IdentityApiDoc;
use utoipa::OpenApi;

#[test]
fn federation_public_error_codes_are_stable_and_unique() {
    let codes = [
        EMAIL_NOT_VERIFIED,
        TENANT_CONTEXT_REQUIRED,
        TENANT_MISMATCH,
        PROVIDER_TYPE_MISMATCH,
        MISSING_EMAIL,
        PROVIDER_INACTIVE,
        INVALID_PROVIDER_ID,
        DOMAIN_NOT_VERIFIED,
        DOMAIN_NOT_FOUND,
        PROVIDER_NOT_FOUND,
        CONNECTOR_NOT_FOUND,
        IDENTITY_CONFLICT,
        PRINCIPAL_CONFLICT,
        INVALID_ID_TOKEN,
        JWKS_MISSING,
        OIDC_DISCOVERY_INVALID,
        OIDC_DISCOVERY_FETCH_FAILED,
        INVALID_SAML_RESPONSE,
        SAML_METADATA_INVALID,
        SAML_METADATA_FETCH_FAILED,
        SAML_SIGNATURE_INVALID,
    ];

    assert!(codes.iter().all(|code| !code.is_empty()));
    let unique: HashSet<&str> = codes.iter().copied().collect();
    assert_eq!(unique.len(), codes.len());
}

#[test]
fn openapi_includes_federation_and_scim_contracts() {
    let openapi = serde_json::from_str::<serde_json::Value>(
        &IdentityApiDoc::openapi()
            .to_json()
            .expect("OpenAPI document should serialize"),
    )
    .expect("OpenAPI document should be valid JSON");

    let paths = &openapi["paths"];
    assert!(paths.get("/tenants/{tenantId}/domains").is_some());
    assert!(
        paths
            .get("/tenants/{tenantId}/identity-providers/{providerId}/oidc/discovery")
            .is_some()
    );
    assert!(
        paths
            .get("/tenants/{tenantId}/identity-providers/{providerId}/saml/metadata")
            .is_some()
    );
    assert!(paths.get("/tenants/{tenantId}/jit-provisioning").is_some());
    assert!(paths.get("/tenants/{tenantId}/scim-connectors").is_some());
    assert!(
        paths
            .get("/tenants/{tenantId}/scim-connectors/{connectorId}")
            .is_some()
    );

    let scim_create_responses = &paths["/tenants/{tenantId}/scim-connectors"]["post"]["responses"];
    assert!(scim_create_responses.get("400").is_some());
    assert!(scim_create_responses.get("401").is_some());
}
