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
        let exposes_billing_segment = path
            .trim_start_matches('/')
            .split('/')
            .any(|segment| segment == "billing");
        assert!(
            !exposes_billing_segment,
            "Identity OpenAPI must not expose Billing path: {path}"
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
