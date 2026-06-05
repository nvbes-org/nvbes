use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};

use crate::{app::AppState, http::error::AppError};

pub fn router(_state: &AppState) -> Router<AppState> {
    Router::new()
        .route("/auth/logout", post(logout))
        .route("/auth/me", get(me))
        .layer(axum::middleware::from_fn(
            nvbes_core::security::no_cache_headers,
        ))
}

async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Result<Response, AppError> {
    let token = crate::http::request::bearer_token(&headers)?;
    let result = crate::domains::auth::logout(&state.db, &token, Some(&headers)).await?;
    let mut response = (StatusCode::OK, Json(result)).into_response();
    nvbes_core::security::insert_clear_site_data_header(response.headers_mut());
    Ok(response)
}

async fn me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<crate::domains::auth::types::MeResult>, AppError> {
    let token = crate::http::request::bearer_token(&headers)?;
    let result = crate::domains::auth::me(&state.db, &token, Some(&headers)).await?;
    Ok(Json(result))
}
