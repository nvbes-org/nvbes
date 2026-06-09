use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, Method, StatusCode},
    middleware::Next,
    response::Response,
};
use nvbes_core::idempotency::{
    build_request_signature, derive_scope, fetch_idempotency_response, insert_idempotency_response,
    validate_key,
};

use crate::app::AppState;
use crate::http::error::AppError;

const MAX_BUFFERED_BODY: usize = 1024 * 1024;

pub async fn idempotency_guard(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: axum::http::Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    if !is_mutating_method(request.method()) {
        return Ok(next.run(request).await);
    }

    let key = match headers.get("Idempotency-Key") {
        Some(v) => v.to_str().map_err(|_| {
            AppError::bad_request(
                "invalid_idempotency_key",
                "Invalid Idempotency-Key header encoding",
            )
        })?,
        None => return Ok(next.run(request).await),
    };

    let key =
        validate_key(key).map_err(|msg| AppError::bad_request("invalid_idempotency_key", &msg))?;

    let scope = derive_scope(&headers, request.method(), request.uri());

    let (parts, body) = request.into_parts();
    let body_bytes = axum::body::to_bytes(body, MAX_BUFFERED_BODY)
        .await
        .map_err(|_| {
            AppError::bad_request(
                "body_too_large",
                "Request body exceeds size limit for idempotency buffering",
            )
        })?;

    let signature = build_request_signature(&parts.method, &parts.uri, &body_bytes);

    if let Some(stored) = fetch_idempotency_response(&state.redis, key, &scope)
        .await
        .map_err(|e| {
            AppError::internal(
                "idempotency_lookup_error",
                format!("Failed to check idempotency key: {e}"),
            )
        })?
    {
        if stored.request_hash == signature {
            return Ok(Response::builder()
                .status(
                    StatusCode::from_u16(stored.response_status as u16)
                        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                )
                .header("Content-Type", "application/json")
                .header("Idempotency-Replayed", "true")
                .body(Body::from(stored.response_body))
                .unwrap());
        }
        return Err(AppError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "idempotency_key_reuse",
            "This idempotency key was already used with a different request body. Use a unique key per distinct request.",
        ));
    }

    let request = axum::http::Request::from_parts(parts, Body::from(body_bytes.clone()));
    let response = next.run(request).await;

    let (resp_parts, resp_body) = response.into_parts();
    let resp_bytes = axum::body::to_bytes(resp_body, MAX_BUFFERED_BODY)
        .await
        .unwrap_or_default();
    let resp_status = resp_parts.status.as_u16() as i32;

    let _ = insert_idempotency_response(
        &state.redis,
        key,
        &scope,
        &signature,
        resp_status,
        &resp_bytes,
    )
    .await;

    Ok(Response::from_parts(resp_parts, Body::from(resp_bytes)))
}

fn is_mutating_method(method: &Method) -> bool {
    matches!(method, &Method::POST | &Method::PUT | &Method::PATCH)
}
