use crate::http::openapi::IdentityApiDoc;
use utoipa::OpenApi;

#[test]
fn openapi_excludes_billing_surface_from_identity() {
    let openapi = serde_json::from_str::<serde_json::Value>(
        &IdentityApiDoc::openapi()
            .to_json()
            .expect("OpenAPI document should serialize"),
    )
    .expect("OpenAPI document should be valid JSON");

    let paths = openapi["paths"]
        .as_object()
        .expect("OpenAPI document should declare paths");
    for path in paths.keys() {
        let is_account_billing_facade = path.starts_with("/account/billing/");
        let exposes_billing_segment = path
            .trim_start_matches('/')
            .split('/')
            .any(|segment| segment == "billing");
        assert!(
            !exposes_billing_segment || is_account_billing_facade,
            "Account OpenAPI must expose Billing only through the Account facade: {path}"
        );
    }

    let tags = openapi["tags"]
        .as_array()
        .expect("OpenAPI document should declare tags");
    assert!(
        !tags
            .iter()
            .filter_map(|tag| tag.get("name").and_then(serde_json::Value::as_str))
            .any(|name| name == "billing"),
        "Identity OpenAPI must not expose a Billing tag"
    );

    let schemas = openapi["components"]["schemas"]
        .as_object()
        .expect("OpenAPI document should declare schemas");
    for schema in [
        "BillingAccountView",
        "BillingPortalCapabilities",
        "BillingPortalCreditView",
        "BillingPortalInvoiceView",
        "BillingPortalView",
        "BillingWebhookResponse",
        "CheckoutSessionResponse",
        concat!("Enterprise", "BillingPlan"),
        concat!("Enterprise", "BillingResponse"),
        "EnterpriseInvoice",
        "PortalSessionResponse",
    ] {
        assert!(
            !schemas.contains_key(schema),
            "Identity OpenAPI must not expose Billing schema: {schema}"
        );
    }
}

#[test]
fn openapi_includes_account_billing_facade_routes() {
    let openapi = serde_json::from_str::<serde_json::Value>(
        &IdentityApiDoc::openapi()
            .to_json()
            .expect("OpenAPI document should serialize"),
    )
    .expect("OpenAPI document should be valid JSON");

    let paths = openapi["paths"]
        .as_object()
        .expect("OpenAPI document should declare paths");
    for path in [
        "/account/billing/workspaces/{workspaceId}/overview",
        "/account/billing/workspaces/{workspaceId}/portal",
        "/account/billing/workspaces/{workspaceId}/checkout",
    ] {
        assert!(
            paths.contains_key(path),
            "Account OpenAPI must expose billing facade path: {path}"
        );
    }

    let schemas = openapi["components"]["schemas"]
        .as_object()
        .expect("OpenAPI document should declare schemas");
    for schema in [
        "AccountBillingOverview",
        "AccountBillingPortalView",
        "AccountBillingSession",
    ] {
        assert!(
            schemas.contains_key(schema),
            "Account OpenAPI must expose account billing schema: {schema}"
        );
    }
}
