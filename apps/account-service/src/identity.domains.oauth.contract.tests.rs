use crate::http::openapi::IdentityApiDoc;
use utoipa::OpenApi;

#[test]
fn openapi_excludes_service_account_management_routes() {
    let openapi = serde_json::from_str::<serde_json::Value>(
        &IdentityApiDoc::openapi()
            .to_json()
            .expect("OpenAPI document should serialize"),
    )
    .expect("OpenAPI document should be valid JSON");

    let paths = &openapi["paths"];
    for path in [
        "/workspaces/{workspaceId}/service-accounts",
        "/workspaces/{workspaceId}/service-accounts/{serviceAccountId}",
        "/workspaces/{workspaceId}/service-accounts/{serviceAccountId}/oauth-clients",
        "/workspaces/{workspaceId}/service-accounts/{serviceAccountId}/oauth-clients:attach",
    ] {
        assert!(paths.get(path).is_none(), "legacy path leaked: {path}");
    }
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
fn openapi_excludes_workspace_management_schema() {
    let openapi = serde_json::from_str::<serde_json::Value>(
        &IdentityApiDoc::openapi()
            .to_json()
            .expect("OpenAPI document should serialize"),
    )
    .expect("OpenAPI document should be valid JSON");

    assert!(
        openapi["components"]["schemas"]
            .get("WorkspaceView")
            .is_none(),
        "WorkspaceView should not be published by Account OpenAPI"
    );
}
