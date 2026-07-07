use crate::http::openapi::IdentityApiDoc;
use utoipa::OpenApi;

#[test]
fn openapi_includes_service_account_management_routes() {
    let openapi = serde_json::from_str::<serde_json::Value>(
        &IdentityApiDoc::openapi()
            .to_json()
            .expect("OpenAPI document should serialize"),
    )
    .expect("OpenAPI document should be valid JSON");

    let paths = &openapi["paths"];
    assert!(
        paths
            .get("/workspaces/{workspaceId}/service-accounts")
            .is_some()
    );
    assert!(
        paths
            .get("/workspaces/{workspaceId}/service-accounts/{serviceAccountId}")
            .is_some()
    );
    assert!(
        paths
            .get("/workspaces/{workspaceId}/service-accounts/{serviceAccountId}/oauth-clients")
            .is_some()
    );
    assert!(
        paths
            .get(
                "/workspaces/{workspaceId}/service-accounts/{serviceAccountId}/oauth-clients:attach"
            )
            .is_some()
    );
}

#[test]
fn openapi_includes_stable_introspection_fields() {
    let openapi = serde_json::from_str::<serde_json::Value>(
        &IdentityApiDoc::openapi()
            .to_json()
            .expect("OpenAPI document should serialize"),
    )
    .expect("OpenAPI document should be valid JSON");

    let properties = &openapi["components"]["schemas"]["IntrospectionResponse"]["properties"];
    for field in [
        "active",
        "principal_type",
        "sub",
        "client_id",
        "role",
        "tenant_id",
        "organization_id",
        "workspace_id",
        "amr",
        "act",
        "actor_principal_type",
        "actor_role",
        "actor_tenant_id",
        "actor_organization_id",
        "actor_workspace_id",
    ] {
        assert!(
            properties.get(field).is_some(),
            "missing schema field: {field}"
        );
    }
}

#[test]
fn openapi_exposes_workspace_owner_principal_id() {
    let openapi = serde_json::from_str::<serde_json::Value>(
        &IdentityApiDoc::openapi()
            .to_json()
            .expect("OpenAPI document should serialize"),
    )
    .expect("OpenAPI document should be valid JSON");

    let properties = &openapi["components"]["schemas"]["WorkspaceView"]["properties"];
    assert!(
        properties.get("owner_principal_id").is_some(),
        "missing schema field: owner_principal_id"
    );
}
