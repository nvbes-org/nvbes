use axum::http::{HeaderName, HeaderValue, Method, header};
use tower_http::cors::{AllowOrigin, Any, CorsLayer};

use crate::oauth::clients::ClientRegistry;

pub(super) fn clients(registry: &ClientRegistry) -> CorsLayer {
    let origins: Vec<HeaderValue> = registry
        .browser_origins()
        .into_iter()
        .filter_map(|origin| origin.parse().ok())
        .collect();
    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            HeaderName::from_static("dpop"),
        ])
        .expose_headers([
            header::WWW_AUTHENTICATE,
            HeaderName::from_static("dpop-nonce"),
        ])
}

pub(super) fn metadata() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET])
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode},
        routing::post,
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn preflight_uses_exact_origins_without_browser_credentials() {
        let registry = ClientRegistry::from_json(r#"[{
          "client_id":"account-web","display_name":"Account",
          "redirect_uris":["https://account.example/callback"],"post_logout_redirect_uris":[],
          "resources":{"https://account-api.example":{"audience":"nvbes-account-service","scopes":["account:read"]}},
          "allow_refresh":true,"require_dpop":true
        }]"#, false).unwrap();
        let app = Router::new()
            .route("/oauth/token", post(|| async { StatusCode::UNAUTHORIZED }))
            .layer(clients(&registry));
        for (origin, allowed) in [
            ("https://account.example", true),
            ("https://account.example.attacker.test", false),
            ("http://account.example", false),
            ("null", false),
        ] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("OPTIONS")
                        .uri("/oauth/token")
                        .header("origin", origin)
                        .header("access-control-request-method", "POST")
                        .header("access-control-request-headers", "dpop,content-type")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(
                response
                    .headers()
                    .get("access-control-allow-origin")
                    .is_some(),
                allowed
            );
            assert!(
                !response
                    .headers()
                    .contains_key("access-control-allow-credentials")
            );
            assert!(
                response.headers()["vary"]
                    .to_str()
                    .unwrap()
                    .contains("origin")
            );
        }
        let response = app
            .oneshot(
                Request::post("/oauth/token")
                    .header("origin", "https://account.example")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(
            response.headers()["access-control-allow-origin"],
            "https://account.example"
        );
        assert!(
            response.headers()["access-control-expose-headers"]
                .to_str()
                .unwrap()
                .contains("dpop-nonce")
        );
    }
}
