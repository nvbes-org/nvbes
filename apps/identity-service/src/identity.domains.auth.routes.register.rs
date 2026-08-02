use crate::domains::auth::{exposed_credentials, onboarding};
use crate::{app::AppState, http::error::AppError};
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, header},
    response::{IntoResponse, Response},
    routing::post,
};
use nvbes_core::http::error::ErrorEnvelope;
use nvbes_product_analytics::ProductAnalyticsEvent;
use nvbes_region::{country_code_to_data_region, detect_profile_from_country_code};
use serde::Deserialize;
use std::time::{Duration, Instant};
use tracing::{info, warn};
use utoipa::ToSchema;

#[path = "identity.domains.auth.routes.register.availability.rs"]
pub(crate) mod availability;
#[cfg(test)]
#[path = "identity.domains.auth.routes.register.tests.rs"]
mod tests;

const REGISTER_REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/register", post(register))
        .route(
            "/registration/availability",
            post(availability::registration_availability),
        )
        .route("/verify-email", post(verify_email))
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) struct RegisterRequest {
    email: String,
    password: String,
    pow_nonce: String,
    pow_solution: String,
    #[serde(default)]
    legal_documents_accepted: bool,
    #[serde(default)]
    marketing_emails_accepted: bool,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) struct VerifyEmailRequest {
    token: String,
}

fn register_input_from_request(
    request: RegisterRequest,
    data_region: String,
    ip: Option<String>,
    user_agent: Option<String>,
) -> crate::domains::auth::types::RegisterInput {
    crate::domains::auth::types::RegisterInput {
        email: request.email,
        password: request.password,
        data_region: Some(data_region),
        ip,
        user_agent,
        legal_documents_accepted: request.legal_documents_accepted,
        marketing_emails_accepted: request.marketing_emails_accepted,
    }
}

#[utoipa::path(
    post,
    path = "/auth/register",
    tag = "auth",
    request_body = RegisterRequest,
    responses(
        (status = 200, description = "Registration successful", body = crate::domains::auth::types::RegisterResult),
        (status = 400, description = "Validation error", body = ErrorEnvelope),
        (status = 409, description = "Email already exists", body = ErrorEnvelope),
        (status = 429, description = "Rate limited", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn register(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<RegisterRequest>,
) -> Result<Response, AppError> {
    let started_at = Instant::now();
    let secure_cookie = state.config.environment != "development";
    let enrollment_max_age = (state.config.auth_verification_ttl_hours * 60 * 60).max(0);
    match tokio::time::timeout(
        REGISTER_REQUEST_TIMEOUT,
        register_inner(state, headers, request),
    )
    .await
    {
        Ok(Ok(result)) => {
            info!(
                elapsed_ms = started_at.elapsed().as_millis() as u64,
                "auth_register_completed"
            );
            registration_response(result, secure_cookie, enrollment_max_age)
        }
        Ok(Err(error)) => {
            warn!(
                elapsed_ms = started_at.elapsed().as_millis() as u64,
                code = %error.code,
                status = %error.status,
                "auth_register_failed"
            );
            Err(error)
        }
        Err(_) => {
            warn!(
                timeout_ms = REGISTER_REQUEST_TIMEOUT.as_millis() as u64,
                "auth_register_timeout"
            );
            Err(AppError::internal(
                "register_timeout",
                "Registration took too long. Please try again.",
            ))
        }
    }
}

fn registration_response(
    Json(result): Json<crate::domains::auth::types::RegisterResult>,
    secure_cookie: bool,
    enrollment_max_age: i64,
) -> Result<Response, AppError> {
    let enrollment_token = result.registration_enrollment_token.clone();
    let mut response = Json(result).into_response();
    response.headers_mut().append(
        header::SET_COOKIE,
        crate::http::cookies::registration_enrollment_cookie(
            &enrollment_token,
            enrollment_max_age,
            secure_cookie,
        )?,
    );
    Ok(response)
}

async fn register_inner(
    state: AppState,
    headers: HeaderMap,
    request: RegisterRequest,
) -> Result<Json<crate::domains::auth::types::RegisterResult>, AppError> {
    info!("auth_register_started");

    crate::domains::auth::challenge_proof::require_pow_solution(
        &state.db,
        Some(request.pow_nonce.as_str()),
        Some(request.pow_solution.as_str()),
    )
    .await?;
    info!("auth_register_pow_verified");

    nvbes_core::limiter::check_dual_rate_limit(
        &state.redis,
        &headers,
        "auth_register",
        &request.email,
        8,
        4,
        std::time::Duration::from_secs(60),
    )
    .await?;
    info!("auth_register_rate_limit_passed");

    let ip = crate::http::request::client_ip(&headers);
    let detected_country = registration_country_from_headers(&headers, &state.config.environment);
    let Some(country_code) = detected_country else {
        return Err(AppError::bad_request(
            "region_detection_failed",
            "Unable to detect a supported region from this connection. Please try again.",
        ));
    };
    info!(country = %country_code, "auth_register_region_resolved");
    let data_region = country_code_to_data_region(&country_code)
        .expect("supported country must map to a region")
        .as_str()
        .to_string();

    if !request.legal_documents_accepted {
        return Err(AppError::bad_request(
            "legal_documents_required",
            "Legal documents must be accepted to create an account.",
        ));
    }
    exposed_credentials::check_new_password(&headers)?;
    info!("auth_register_input_validated");

    let result = onboarding::register(
        &state.db,
        &state.redis,
        &state.config,
        register_input_from_request(
            request,
            data_region,
            ip,
            crate::http::request::user_agent(&headers),
        ),
    )
    .await?;
    info!("auth_register_onboarding_completed");

    let (distinct_id, session_id) = crate::http::request::product_analytics_correlation(&headers);
    state.product_analytics.capture(
        ProductAnalyticsEvent::user("auth.signup_completed", result.user.id)
            .correlation(distinct_id, session_id)
            .property("country", country_code),
    );
    info!("auth_register_analytics_captured");

    Ok(Json(result))
}

fn registration_country_from_headers(headers: &HeaderMap, environment: &str) -> Option<String> {
    crate::http::request::region_from_headers(headers)
        .and_then(|country| {
            detect_profile_from_country_code(&country)
                .map(|profile| profile.country_code.to_string())
        })
        .or_else(|| matches!(environment, "development" | "test").then(|| "FR".to_string()))
}

#[utoipa::path(
    post,
    path = "/auth/verify-email",
    tag = "auth",
    request_body = VerifyEmailRequest,
    responses(
        (status = 200, description = "Email verified", body = crate::domains::auth::types::VerifyEmailResult),
        (status = 400, description = "Invalid or expired token", body = ErrorEnvelope),
        (status = 429, description = "Rate limited", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn verify_email(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<VerifyEmailRequest>,
) -> Result<Response, AppError> {
    nvbes_core::limiter::check_dual_rate_limit(
        &state.redis,
        &headers,
        "auth_verify_email",
        &request.token,
        30,
        10,
        std::time::Duration::from_secs(60),
    )
    .await?;

    let result = onboarding::verify_email(
        &state.db,
        &state.redis,
        crate::domains::auth::types::VerifyEmailInput {
            token: request.token,
        },
    )
    .await?;

    let (distinct_id, session_id) = crate::http::request::product_analytics_correlation(&headers);
    state.product_analytics.capture(
        ProductAnalyticsEvent::user("auth.email_verified", result.user.id)
            .correlation(distinct_id, session_id),
    );

    let secure_cookie = state.config.environment != "development";
    let mut response = Json(result).into_response();
    response.headers_mut().append(
        header::SET_COOKIE,
        crate::http::cookies::clear_registration_enrollment_cookie(secure_cookie)?,
    );
    Ok(response)
}
