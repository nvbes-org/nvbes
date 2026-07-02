use crate::http::openapi::IdentityApiDoc;
use utoipa::OpenApi;

#[test]
fn openapi_includes_billing_portal_contracts() {
    let openapi = serde_json::from_str::<serde_json::Value>(
        &IdentityApiDoc::openapi()
            .to_json()
            .expect("OpenAPI document should serialize"),
    )
    .expect("OpenAPI document should be valid JSON");

    let paths = &openapi["paths"];
    assert!(paths.get("/billing/portal/capabilities").is_some());
    assert!(
        paths
            .get("/workspaces/{workspaceId}/billing/portal/view")
            .is_some()
    );

    let schemas = &openapi["components"]["schemas"];
    for schema in [
        "BillingPortalCapabilities",
        "BillingPortalView",
        "BillingPortalInvoiceView",
        "BillingPortalCreditView",
    ] {
        assert!(schemas.get(schema).is_some(), "missing schema: {schema}");
    }

    let portal_session_properties = &schemas["PortalSessionResponse"]["properties"];
    assert!(
        portal_session_properties
            .get("provider_customer_id")
            .is_some()
    );
    assert!(
        portal_session_properties
            .get("stripe_customer_id")
            .is_some()
    );

    let checkout_session_properties = &schemas["CheckoutSessionResponse"]["properties"];
    assert!(
        checkout_session_properties
            .get("provider_product_id")
            .is_some()
    );
    assert!(
        checkout_session_properties
            .get("provider_price_id")
            .is_some()
    );

    let webhook_response_properties = &schemas["BillingWebhookResponse"]["properties"];
    assert!(webhook_response_properties.get("provider").is_some());
    assert!(
        webhook_response_properties
            .get("provider_event_id")
            .is_some()
    );

    let billing_account_properties = &schemas["BillingAccountView"]["properties"];
    assert!(
        billing_account_properties
            .get("provider_customer_id")
            .is_some()
    );
    assert!(
        billing_account_properties
            .get("stripe_customer_id")
            .is_some()
    );

    let capabilities_properties = &schemas["BillingPortalCapabilities"]["properties"];
    for field in [
        "exposes_provider_secret_ids",
        "payment_method_update_flow",
        "payment_method_changes_delegated_to_provider",
        "automatically_updates_payment_method_references",
    ] {
        assert!(
            capabilities_properties.get(field).is_some(),
            "missing capabilities field: {field}"
        );
    }
}
