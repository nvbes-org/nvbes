use axum::Router;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::app::AppState;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "nvbes Drive API",
        version = "0.1.0",
        description = "File storage, sharing, and collaboration API",
        contact(name = "nvbes", url = "https://nvbes.fr"),
        license(name = "UNLICENSED"),
    ),
    servers(
        (url = "https://drive.nvbes.fr", description = "Production"),
        (url = "http://localhost:8081", description = "Development"),
    ),
    tags(
        (name = "files", description = "File and folder management"),
        (name = "uploads", description = "File upload workflow"),
        (name = "share-links", description = "Share link management"),
        (name = "public-shares", description = "Public share access"),
        (name = "quotas", description = "Storage quotas"),
        (name = "billing", description = "Billing & subscriptions"),
        (name = "audit", description = "Audit events"),
        (name = "privacy", description = "Data privacy & export"),
        (name = "api-keys", description = "Legacy API key management (list/revoke only)"),
        (name = "public-api", description = "Public REST API v1"),
    ),
    paths(
        crate::domains::public_api::v1_handlers::me,
        crate::domains::public_api::v1_handlers::list_workspaces,
        crate::domains::public_api::v1_handlers::list_objects,
        crate::domains::public_api::v1_handlers::create_folder,
        crate::domains::public_api::v1_handlers::rename_object,
        crate::domains::public_api::v1_handlers::move_object,
        crate::domains::public_api::v1_handlers::trash_object,
        crate::domains::public_api::v1_handlers::create_download_url,
        crate::domains::public_api::v1_handlers::create_upload,
        crate::domains::public_api::v1_handlers::complete_upload,
        crate::domains::public_api::v1_handlers::cancel_upload,
        crate::domains::public_api::v1_handlers::list_share_links,
        crate::domains::public_api::v1_handlers::create_share_link,
        crate::domains::public_api::v1_handlers::update_share_link,
        crate::domains::public_api::v1_handlers::revoke_share_link,
        crate::domains::public_api::v1_handlers::get_quota,
        crate::domains::public_api::v1_handlers::list_audit_events,
        crate::domains::billing::routes::manage::get_billing,
        crate::domains::billing::routes::manage::create_checkout_session,
        crate::domains::billing::routes::manage::create_portal_session,
        crate::domains::billing::routes::manage::get_usage,
        crate::domains::billing::routes::manage::get_invoice_estimate,
        crate::domains::billing::routes::webhooks::handle_webhook,
    ),
)]
pub struct DriveApiDoc;

pub fn openapi_routes() -> Router<AppState> {
    SwaggerUi::new("/docs")
        .url("/api/openapi.json", DriveApiDoc::openapi())
        .into()
}

#[cfg(test)]
mod tests {
    use super::DriveApiDoc;
    use utoipa::OpenApi;

    #[test]
    fn openapi_does_not_expose_api_key_creation_route() {
        let openapi = serde_json::from_str::<serde_json::Value>(
            &DriveApiDoc::openapi()
                .to_json()
                .expect("OpenAPI document should serialize"),
        )
        .expect("OpenAPI document should be valid JSON");

        let path = &openapi["paths"]["/workspaces/{workspaceId}/api-keys"];
        assert!(
            path.get("post").is_none(),
            "legacy api-key creation must stay absent from openapi"
        );
    }

    #[test]
    fn openapi_includes_provider_neutral_billing_contracts() {
        let openapi = serde_json::from_str::<serde_json::Value>(
            &DriveApiDoc::openapi()
                .to_json()
                .expect("OpenAPI document should serialize"),
        )
        .expect("OpenAPI document should be valid JSON");

        let paths = &openapi["paths"];
        assert!(paths.get("/workspaces/{workspaceId}/billing").is_some());
        assert!(
            paths
                .get("/workspaces/{workspaceId}/billing/checkout")
                .is_some()
        );
        assert!(
            paths
                .get("/workspaces/{workspaceId}/billing/portal")
                .is_some()
        );

        let schemas = &openapi["components"]["schemas"];
        let checkout_properties = &schemas["CheckoutSessionResponse"]["properties"];
        assert!(checkout_properties.get("provider").is_some());
        assert!(checkout_properties.get("provider_customer_id").is_some());
        assert!(checkout_properties.get("provider_product_id").is_some());
        assert!(checkout_properties.get("provider_price_id").is_some());
        assert!(checkout_properties.get("stripe_customer_id").is_some());
        let checkout_required = schemas["CheckoutSessionResponse"]["required"]
            .as_array()
            .expect("CheckoutSessionResponse should declare required fields");
        for legacy_field in ["stripe_customer_id", "stripe_price_id"] {
            assert!(
                !checkout_required
                    .iter()
                    .any(|field| field.as_str() == Some(legacy_field)),
                "legacy field must stay optional: {legacy_field}"
            );
        }

        let portal_properties = &schemas["PortalSessionResponse"]["properties"];
        assert!(portal_properties.get("provider").is_some());
        assert!(portal_properties.get("provider_customer_id").is_some());
        assert!(portal_properties.get("stripe_customer_id").is_some());
        let portal_required = schemas["PortalSessionResponse"]["required"]
            .as_array()
            .expect("PortalSessionResponse should declare required fields");
        assert!(
            !portal_required
                .iter()
                .any(|field| field.as_str() == Some("stripe_customer_id")),
            "legacy portal stripe_customer_id must stay optional"
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
    }
}
