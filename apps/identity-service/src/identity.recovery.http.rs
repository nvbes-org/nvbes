use crate::{auth, mfa_crypto::MfaCrypto, recovery, recovery_delivery};

#[cfg(all(test, feature = "database-tests"))]
#[path = "identity.recovery.http.tests.rs"]
mod tests;
use axum::{
    Json, Router,
    extract::{ConnectInfo, DefaultBodyLimit, Request, State},
    http::{HeaderMap, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use hmac::{Hmac, KeyInit, Mac};
use nvbes_identity_service::{
    browser::BrowserSecurity,
    rate_limits::{Category, LimitError, RateLimiter},
};
use sha2::Sha256;
use sqlx::PgPool;
use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::Duration,
};

#[derive(Clone)]
struct RecoveryHttp {
    db: PgPool,
    browser: BrowserSecurity,
    limiter: RateLimiter,
    crypto: Arc<MfaCrypto>,
    base_url: String,
    permits: Arc<tokio::sync::Semaphore>,
}

pub fn router(
    db: PgPool,
    browser: BrowserSecurity,
    limiter: RateLimiter,
    crypto: Arc<MfaCrypto>,
    base_url: String,
) -> Router {
    let state = RecoveryHttp {
        db,
        browser,
        limiter,
        crypto,
        base_url,
        permits: Arc::new(tokio::sync::Semaphore::new(16)),
    };
    Router::new()
        .route("/oauth/password-recovery/context", get(context))
        .route("/oauth/password-recovery/request", post(request))
        .route("/oauth/password-recovery/reset", post(reset))
        .layer(DefaultBodyLimit::max(4096))
        .route_layer(middleware::from_fn_with_state(state.clone(), protect))
        .with_state(state)
}

fn error(status: StatusCode, code: &'static str) -> Response {
    let mut response = (status, Json(serde_json::json!({"error":code}))).into_response();
    if status == StatusCode::TOO_MANY_REQUESTS {
        response
            .headers_mut()
            .insert(header::RETRY_AFTER, "900".parse().unwrap());
    }
    response
}
async fn quota(state: &RecoveryHttp, category: Category, subject: &str) -> Result<(), Response> {
    match tokio::time::timeout(
        Duration::from_secs(2),
        state.limiter.check(&state.db, category, subject),
    )
    .await
    {
        Ok(Ok(())) => Ok(()),
        Ok(Err(LimitError::Exceeded)) => Err(error(StatusCode::TOO_MANY_REQUESTS, "rate_limited")),
        _ => Err(error(StatusCode::SERVICE_UNAVAILABLE, "unavailable")),
    }
}
fn csrf(browser: &str) -> Hmac<Sha256> {
    let mut mac =
        Hmac::<Sha256>::new_from_slice(browser.as_bytes()).expect("HMAC accepts browser tokens");
    mac.update(b"nvbes:password-recovery:v1");
    mac
}
async fn protect(State(state): State<RecoveryHttp>, request: Request, next: Next) -> Response {
    let mut response = protected(&state, request, next).await;
    response
        .headers_mut()
        .extend(state.browser.response_headers());
    response
}
async fn protected(state: &RecoveryHttp, request: Request, next: Next) -> Response {
    let Ok(_permit) = state.permits.try_acquire() else {
        return error(StatusCode::SERVICE_UNAVAILABLE, "unavailable");
    };
    let Some(peer) = request.extensions().get::<ConnectInfo<SocketAddr>>() else {
        return error(StatusCode::SERVICE_UNAVAILABLE, "unavailable");
    };
    let source = match peer.0.ip() {
        IpAddr::V4(ip) => ip.to_string(),
        IpAddr::V6(ip) => match ip.to_ipv4_mapped() {
            Some(ip) => ip.to_string(),
            None => {
                let [a, b, c, d, _, _, _, _] = ip.segments();
                format!("{}/64", std::net::Ipv6Addr::new(a, b, c, d, 0, 0, 0, 0))
            }
        },
    };
    let mutation = request.method() == axum::http::Method::POST;
    let category = if mutation {
        Category::RecoverySource
    } else {
        Category::ProtocolSource
    };
    if let Err(response) = quota(state, category, &source).await {
        return response;
    }
    if mutation {
        let verified = state
            .browser
            .verify_mutation(request.method(), request.headers())
            .is_ok();
        let browser = state
            .browser
            .browser_token(request.headers())
            .ok()
            .flatten();
        let proof = request
            .headers()
            .get("x-csrf-token")
            .and_then(|h| h.to_str().ok())
            .and_then(|v| URL_SAFE_NO_PAD.decode(v).ok());
        if !verified
            || !browser
                .zip(proof)
                .is_some_and(|(browser, proof)| csrf(&browser).verify_slice(&proof).is_ok())
        {
            return error(StatusCode::FORBIDDEN, "invalid_browser_request");
        }
    }
    match tokio::time::timeout(Duration::from_secs(5), next.run(request)).await {
        Ok(response) => response,
        Err(_) => error(StatusCode::SERVICE_UNAVAILABLE, "unavailable"),
    }
}

async fn context(State(state): State<RecoveryHttp>, headers: HeaderMap) -> Response {
    let Ok(cookie) = state.browser.existing_or_new_browser_cookie(&headers) else {
        return error(StatusCode::BAD_REQUEST, "invalid_request");
    };
    let proof = URL_SAFE_NO_PAD.encode(csrf(&cookie.token).finalize().into_bytes());
    let mut response = Json(serde_json::json!({"csrf_token":proof})).into_response();
    response
        .headers_mut()
        .append(header::SET_COOKIE, cookie.header);
    response
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RecoveryRequest {
    email: String,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ResetRequest {
    token: String,
    password: String,
}

async fn request(
    State(state): State<RecoveryHttp>,
    body: Result<Json<RecoveryRequest>, axum::extract::rejection::JsonRejection>,
) -> Response {
    let started = tokio::time::Instant::now();
    let Ok(Json(body)) = body else {
        return error(StatusCode::BAD_REQUEST, "invalid_request");
    };
    let Ok(email) = auth::normalize_email(&body.email) else {
        return error(StatusCode::BAD_REQUEST, "invalid_request");
    };
    if let Err(response) = quota(&state, Category::RecoveryAccount, &email).await {
        return response;
    }
    let result =
        recovery_delivery::enqueue(&state.db, &state.crypto, &state.base_url, &email).await;
    // A floor reduces the ordinary eligible/unknown timing difference; it does
    // not claim constant latency under database contention or failures.
    tokio::time::sleep_until(started + Duration::from_millis(250)).await;
    match result {
        Ok(_) => (
            StatusCode::ACCEPTED,
            Json(serde_json::json!({"accepted":true})),
        )
            .into_response(),
        Err(error)
            if error
                .downcast_ref::<recovery::RecoveryUnavailable>()
                .is_some() =>
        {
            (
                StatusCode::ACCEPTED,
                Json(serde_json::json!({"accepted":true})),
            )
                .into_response()
        }
        Err(_) => error(StatusCode::SERVICE_UNAVAILABLE, "unavailable"),
    }
}
async fn reset(
    State(state): State<RecoveryHttp>,
    body: Result<Json<ResetRequest>, axum::extract::rejection::JsonRejection>,
) -> Response {
    let Ok(Json(body)) = body else {
        return error(StatusCode::BAD_REQUEST, "invalid_request");
    };
    if body.token.len() != 43 || auth::validate_password(&body.password).is_err() {
        return error(StatusCode::BAD_REQUEST, "invalid_request");
    }
    if let Err(response) = quota(&state, Category::RecoveryToken, &body.token).await {
        return response;
    }
    match recovery::reset_password(&state.db, &body.token, &body.password).await {
        Ok(()) => {
            let mut response =
                Json(serde_json::json!({"reset":true,"must_reauthenticate":true})).into_response();
            response
                .headers_mut()
                .append(header::SET_COOKIE, state.browser.clear_session_cookie());
            response
        }
        Err(failure)
            if failure
                .downcast_ref::<sqlx::Error>()
                .is_some_and(|e| !matches!(e, sqlx::Error::RowNotFound)) =>
        {
            error(StatusCode::SERVICE_UNAVAILABLE, "unavailable")
        }
        Err(_) => error(StatusCode::BAD_REQUEST, "invalid_recovery"),
    }
}
