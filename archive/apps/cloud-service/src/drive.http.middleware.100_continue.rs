use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, Request, header},
    middleware::Next,
    response::Response,
};

use crate::app::AppState;
use crate::http::{error::AppError, request::bearer_token};

pub async fn authenticate_before_body(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    let is_expect_continue = headers
        .get(header::EXPECT)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.eq_ignore_ascii_case("100-continue"))
        .unwrap_or(false);

    if !is_expect_continue {
        return Ok(next.run(request).await);
    }

    let token = bearer_token(&headers)?;

    let _auth = crate::domains::auth::authenticate(&state.db, &token, Some(&headers)).await?;

    Ok(next.run(request).await)
}
