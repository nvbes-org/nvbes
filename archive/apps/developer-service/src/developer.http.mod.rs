use std::{future::Future, net::SocketAddr};

use axum::{
    Json, Router,
    middleware::{from_fn, from_fn_with_state},
    routing::get,
};
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};

use crate::app::DeveloperAppState;

#[path = "developer.http.auth.rs"]
pub mod auth;
#[path = "developer.http.context.rs"]
pub mod context;
#[path = "developer.http.error.rs"]
pub mod error;
#[path = "developer.http.openapi.rs"]
pub mod openapi;
#[path = "developer.http.routes.rs"]
mod routes;
#[path = "developer.http.types.rs"]
pub mod types;

pub fn router(state: DeveloperAppState) -> Router {
    let origins = allowed_origins();
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(move |origin, _| {
            origins.iter().any(|allowed| origin == allowed)
        }))
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
            axum::http::Method::PATCH,
            axum::http::Method::DELETE,
        ])
        .allow_headers([
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
            axum::http::header::ACCEPT,
        ]);

    let protected =
        routes::router().route_layer(from_fn_with_state(state.clone(), auth::authenticate));

    Router::new()
        .route("/health", get(health))
        .merge(openapi::routes())
        .merge(protected)
        .layer(cors)
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(TraceLayer::new_for_http())
        .layer(from_fn(nvbes_core::security::security_headers))
        .with_state(state)
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "developer-service",
    }))
}

pub async fn serve(
    addr: SocketAddr,
    state: DeveloperAppState,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> std::io::Result<()> {
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router(state))
        .with_graceful_shutdown(shutdown)
        .await
}

fn allowed_origins() -> Vec<axum::http::HeaderValue> {
    std::env::var("NVBES_DEVELOPER_ALLOWED_ORIGINS")
        .unwrap_or_else(|_| "http://localhost:5175".to_string())
        .split(',')
        .filter_map(|origin| origin.trim().parse().ok())
        .collect()
}
