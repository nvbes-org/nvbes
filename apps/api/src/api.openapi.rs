use axum::{Json, Router, response::Html, routing::get};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::health::healthz_handler,
        crate::health::readyz_handler,
    ),
    components(
        schemas(
            crate::health::HealthStatus,
            crate::health::ReadinessStatus,
            nvbes_consent::models::CookiePreferences,
            nvbes_consent::models::CookiePreferencesResponse,
            nvbes_consent::models::RecordAgreementRequest,
            nvbes_consent::models::UpdateCookiePreferencesRequest,
            nvbes_consent::models::ReconcileConsentRequest,
            nvbes_consent::models::ConsentStatusResponse,
        )
    ),
    tags(
        (name = "Health", description = "Liveness and readiness probes"),
        (name = "Consent", description = "Legal agreements and cookie preferences with GPC"),
        (name = "Identity", description = "OAuth2 / OIDC authentication and sessions"),
        (name = "Account", description = "Profile, settings, and team management"),
    ),
    info(
        title = "NVBES Unified Monolith API",
        version = "0.1.0",
        description = "Production serverless unified API for NVBES identity, accounts, and consent."
    )
)]
pub struct ApiDoc;

pub fn openapi_spec() -> utoipa::openapi::OpenApi {
    ApiDoc::openapi()
}

pub fn router() -> Router {
    Router::new()
        .route("/api/docs/openapi.json", get(openapi_json_handler))
        .route("/api/docs", get(swagger_ui_html_handler))
}

async fn openapi_json_handler() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

async fn swagger_ui_html_handler() -> Html<&'static str> {
    Html(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <title>NVBES API Documentation</title>
  <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css" />
</head>
<body>
  <div id="swagger-ui"></div>
  <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
  <script>
    window.onload = () => {
      window.ui = SwaggerUIBundle({
        url: '/api/docs/openapi.json',
        dom_id: '#swagger-ui',
      });
    };
  </script>
</body>
</html>"#,
    )
}
