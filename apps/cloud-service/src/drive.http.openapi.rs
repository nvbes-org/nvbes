use axum::Router;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::app::AppState;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "nvbes Cloud Service",
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
    fn openapi_does_not_expose_billing_runtime_contracts() {
        let openapi = serde_json::from_str::<serde_json::Value>(
            &DriveApiDoc::openapi()
                .to_json()
                .expect("OpenAPI document should serialize"),
        )
        .expect("OpenAPI document should be valid JSON");

        let paths = &openapi["paths"];
        assert!(
            paths
                .as_object()
                .expect("OpenAPI paths should be an object")
                .keys()
                .all(|path| !path.contains("/billing")),
            "Drive OpenAPI must not expose Billing runtime paths"
        );
    }
}
