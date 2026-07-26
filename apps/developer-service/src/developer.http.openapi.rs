use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "nvbes Developer Service",
        version = "0.1.0",
        description = "Developer Console, applications, webhooks, credentials, and integration tooling",
        contact(name = "nvbes", url = "https://nvbes.fr"),
        license(name = "UNLICENSED"),
    ),
    servers(
        (url = "https://developer-api.nvbes.fr", description = "Production"),
        (url = "http://localhost:4040", description = "Development"),
    ),
    tags(
        (name = "developer", description = "Developer portal API"),
        (name = "developer-console", description = "Developer Console API"),
    ),
    paths(
        crate::http::routes::portal::me,
        crate::http::routes::portal::list_apps,
        crate::http::routes::portal::create_app,
        crate::http::routes::portal::get_app,
        crate::http::routes::portal::update_redirects,
        crate::http::routes::portal::revoke_app,
        crate::http::routes::portal::list_logs,
        crate::http::routes::webhooks::list_portal_webhooks,
        crate::http::routes::webhooks::create_portal_webhook,
        crate::http::routes::webhooks::delete_portal_webhook,
        crate::http::routes::tools::inspect_token,
        crate::http::routes::tools::exchange_playground_code,
        crate::http::routes::console::context,
        crate::http::routes::console::overview,
        crate::http::routes::oauth::list_oauth_clients,
        crate::http::routes::oauth::list_marketplace_apps,
        crate::http::routes::oauth::submit_marketplace_app,
        crate::http::routes::oauth::review_marketplace_app,
        crate::http::routes::oauth::list_scopes,
        crate::http::routes::oauth::create_scope,
        crate::http::routes::oauth::update_scope,
        crate::http::routes::oauth::delete_scope,
        crate::http::routes::oauth::get_consent_screen,
        crate::http::routes::oauth::upsert_consent_screen,
        crate::http::routes::secrets::list_service_accounts,
        crate::http::routes::secrets::list_secret_versions,
        crate::http::routes::secrets::rotate_secret,
        crate::http::routes::secrets::revoke_secret_version,
        crate::http::routes::webhooks::list_console_webhooks,
        crate::http::routes::webhooks::list_deliveries,
        crate::http::routes::webhooks::replay_delivery,
        crate::http::routes::webhooks::list_api_logs,
        crate::http::routes::tools::debug_token,
        crate::http::routes::tools::get_sandbox,
        crate::http::routes::tools::upsert_sandbox,
        crate::http::routes::tools::reset_sandbox,
        crate::http::routes::tools::list_health_checks,
        crate::http::routes::tools::run_health_checks,
    ),
)]
pub struct DeveloperApiDoc;

pub fn routes() -> axum::Router<crate::app::DeveloperAppState> {
    SwaggerUi::new("/docs")
        .url("/api/openapi.json", DeveloperApiDoc::openapi())
        .into()
}
