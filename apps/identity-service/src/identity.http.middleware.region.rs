use axum::{
    Json,
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use nvbes_region::is_country_allowed;
use serde::Serialize;

use super::super::request::region_from_headers;

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct RegionErrorResponse {
    pub error: &'static str,
    pub message: &'static str,
}

pub async fn region_restriction_guard(req: Request<Body>, next: Next) -> Response {
    if let Some(country_code) = region_from_headers(req.headers()) {
        if is_country_allowed(&country_code) {
            return next.run(req).await;
        } else {
            let body = Json(RegionErrorResponse {
                error: "region_not_allowed",
                message: "Service non disponible dans votre région.",
            });
            return (StatusCode::FORBIDDEN, body).into_response();
        }
    }

    next.run(req).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Router,
        body::Body,
        http::{HeaderValue, Request, StatusCode},
        middleware,
        routing::get,
    };
    use tower::ServiceExt;

    async fn mock_handler() -> &'static str {
        "ok"
    }

    fn test_app() -> Router {
        Router::new()
            .route("/test", get(mock_handler))
            .layer(middleware::from_fn(region_restriction_guard))
    }

    #[tokio::test]
    async fn allows_authorized_countries() {
        let app = test_app();

        for country in [
            "FR", "DE", "GB", "CH", "ES", "IT", "IS", "NO", // Europe
            "US", "CA", // North America
            "JP", "KR", "AU", "NZ", "SG", "MY", "TH", "ID", "PH", "VN", // APAC & SE Asia
            "BR", "AR", "CL", "MX", "CO", "PE", "UY", "CR", "PA", // Latam
            "AE", "IL", "SA", "QA", // Middle East
            "MA", "TN", "EG", "KE", "NG", "ZA", "DZ", "ET", // Africa
        ] {
            let mut req = Request::builder().uri("/test").body(Body::empty()).unwrap();
            req.headers_mut()
                .insert("CF-IPCountry", HeaderValue::from_str(country).unwrap());

            let res = app.clone().oneshot(req).await.unwrap();
            assert_eq!(res.status(), StatusCode::OK);
        }
    }

    #[tokio::test]
    async fn blocks_unauthorized_countries() {
        let app = test_app();

        for country in ["CN", "RU", "IN", "KP", "IR", "SY", "BY"] {
            let mut req = Request::builder().uri("/test").body(Body::empty()).unwrap();
            req.headers_mut()
                .insert("CF-IPCountry", HeaderValue::from_str(country).unwrap());

            let res = app.clone().oneshot(req).await.unwrap();
            assert_eq!(res.status(), StatusCode::FORBIDDEN);
        }
    }

    #[tokio::test]
    async fn allows_requests_without_country_header() {
        let app = test_app();
        let req = Request::builder().uri("/test").body(Body::empty()).unwrap();

        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }
}
