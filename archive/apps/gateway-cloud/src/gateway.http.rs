use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    middleware::from_fn,
    response::IntoResponse,
    routing::{get, post},
};
use serde::Serialize;

use crate::{
    auth::request_context,
    schema::GatewaySchema,
    state::{GatewayRequestContext, GatewayState},
};

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[derive(Clone)]
pub struct HttpState {
    pub schema: GatewaySchema,
    pub gateway: GatewayState,
}

pub fn router(state: GatewayState) -> Router {
    let schema = crate::schema::schema(state.clone());
    Router::new()
        .route("/health", get(health))
        .route("/graphql", post(graphql))
        .layer(from_fn(nvbes_core::security::security_headers))
        .with_state(HttpState {
            schema,
            gateway: state,
        })
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

async fn graphql(
    State(state): State<HttpState>,
    headers: HeaderMap,
    request: GraphQLRequest,
) -> Result<GraphQLResponse, StatusCode> {
    let request_context = request_context(&state.gateway, &headers).await?;
    Ok(state
        .schema
        .execute(
            request
                .into_inner()
                .data::<GatewayRequestContext>(request_context),
        )
        .await
        .into())
}

pub async fn not_found() -> impl IntoResponse {
    StatusCode::NOT_FOUND
}
