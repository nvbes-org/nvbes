use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, Method, StatusCode},
    middleware::Next,
    response::Response,
};
use nvbes_core::idempotency::{build_request_signature, derive_scope, validate_key};

use crate::app::AppState;
use crate::error::AppError;

const MAX_BUFFERED_BODY: usize = 1024 * 1024;

struct StoredIdempotencyResponse {
    request_hash: String,
    response_body: Vec<u8>,
    response_status: i32,
}

pub async fn idempotency_guard(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: axum::http::Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    if !is_mutating_method(request.method()) {
        return Ok(next.run(request).await);
    }

    let Some(key) = headers.get("Idempotency-Key") else {
        return Ok(next.run(request).await);
    };
    let key = key.to_str().map_err(|_| {
        AppError::bad_request(
            "invalid_idempotency_key",
            "Invalid Idempotency-Key header encoding.",
        )
    })?;
    let key = validate_key(key).map_err(|message| {
        AppError::bad_request(
            "invalid_idempotency_key",
            format!("Invalid Idempotency-Key: {message}"),
        )
    })?;

    let scope = derive_scope(&headers, request.method(), request.uri());
    let (parts, body) = request.into_parts();
    let body_bytes = axum::body::to_bytes(body, MAX_BUFFERED_BODY)
        .await
        .map_err(|_| {
            AppError::bad_request(
                "body_too_large",
                "Request body exceeds size limit for idempotency buffering.",
            )
        })?;
    let signature = build_request_signature(&parts.method, &parts.uri, &body_bytes);

    if let Some(stored) = fetch_idempotency_response(&state, key, scope.as_str()).await? {
        if stored.request_hash != signature {
            return Err(AppError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "idempotency_key_reuse",
                "This idempotency key was already used with a different request body.",
            ));
        }
        return replay_response(stored);
    }

    let request = axum::http::Request::from_parts(parts, Body::from(body_bytes));
    let response = next.run(request).await;
    let (response_parts, response_body) = response.into_parts();
    let response_bytes = axum::body::to_bytes(response_body, MAX_BUFFERED_BODY)
        .await
        .unwrap_or_default();
    let response_status = response_parts.status.as_u16() as i32;

    insert_idempotency_response(
        &state,
        key,
        scope.as_str(),
        &signature,
        response_status,
        &response_bytes,
    )
    .await?;

    Ok(Response::from_parts(
        response_parts,
        Body::from(response_bytes),
    ))
}

async fn fetch_idempotency_response(
    state: &AppState,
    key: &str,
    scope: &str,
) -> Result<Option<StoredIdempotencyResponse>, AppError> {
    let row = sqlx::query_as::<_, (String, i32, Vec<u8>)>(
        "SELECT request_hash, response_status, response_body
         FROM idempotency_responses
         WHERE idempotency_key = $1 AND scope = $2 AND expires_at > NOW()",
    )
    .bind(key)
    .bind(scope)
    .fetch_optional(&state.db)
    .await?;

    Ok(row.map(
        |(request_hash, response_status, response_body)| StoredIdempotencyResponse {
            request_hash,
            response_body,
            response_status,
        },
    ))
}

async fn insert_idempotency_response(
    state: &AppState,
    key: &str,
    scope: &str,
    request_hash: &str,
    response_status: i32,
    response_body: &[u8],
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO idempotency_responses (
           idempotency_key, scope, request_hash, response_status, response_body
         ) VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (idempotency_key, scope) DO NOTHING",
    )
    .bind(key)
    .bind(scope)
    .bind(request_hash)
    .bind(response_status)
    .bind(response_body)
    .execute(&state.db)
    .await?;
    Ok(())
}

fn replay_response(stored: StoredIdempotencyResponse) -> Result<Response, AppError> {
    Ok(Response::builder()
        .status(
            StatusCode::from_u16(stored.response_status as u16)
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
        )
        .header("content-type", "application/json")
        .header("idempotency-replayed", "true")
        .body(Body::from(stored.response_body))
        .expect("stored idempotency response should be buildable"))
}

fn is_mutating_method(method: &Method) -> bool {
    matches!(method, &Method::POST | &Method::PUT | &Method::PATCH)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutating_methods_are_guarded() {
        assert!(is_mutating_method(&Method::POST));
        assert!(is_mutating_method(&Method::PATCH));
        assert!(!is_mutating_method(&Method::GET));
    }
}
