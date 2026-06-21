use crate::domains::auth::onboarding;
use crate::{app::AppState, http::error::AppError};
use axum::{
    Json, Router,
    extract::State,
    http::HeaderMap,
    routing::{get, post},
};
use nvbes_core::http::error::ErrorEnvelope;
use nvbes_product_analytics::ProductAnalyticsEvent;
use nvbes_region::{
    country_code_to_data_region, detect_profile_from_country_code, supported_profiles,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[cfg(test)]
#[path = "identity.domains.auth.routes.register.tests.rs"]
mod tests;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/register", post(register))
        .route("/verify-email", post(verify_email))
        .route("/region", get(region))
        .route("/regions", get(supported_regions))
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) struct RegisterRequest {
    email: String,
    password: String,
    firstname: Option<String>,
    lastname: Option<String>,
    username: String,
    birthdate: Option<String>,
    region: Option<String>,
    workspace_name: String,
    #[serde(default)]
    pow_nonce: Option<String>,
    #[serde(default)]
    pow_solution: Option<String>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) struct VerifyEmailRequest {
    token: String,
}

fn register_input_from_request(
    request: RegisterRequest,
    country_code: String,
    data_region: String,
    birthdate: Option<chrono::NaiveDate>,
    ip: Option<String>,
    user_agent: Option<String>,
) -> crate::domains::auth::types::RegisterInput {
    crate::domains::auth::types::RegisterInput {
        email: request.email,
        password: request.password,
        firstname: request.firstname,
        lastname: request.lastname,
        username: request.username,
        birthdate,
        region: Some(country_code),
        data_region: Some(data_region),
        workspace_name: request.workspace_name,
        ip,
        user_agent,
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
) -> Result<Json<crate::domains::auth::types::RegisterResult>, AppError> {
    if state.config.auth_pow_enabled {
        crate::domains::auth::challenge_proof::require_pow_solution(
            &state.db,
            request.pow_nonce.as_deref(),
            request.pow_solution.as_deref(),
        )
        .await?;
    }

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

    let ip = crate::http::request::client_ip(&headers);
    let detected_country =
        crate::http::request::region_from_headers(&headers).and_then(|country| {
            detect_profile_from_country_code(&country)
                .map(|profile| profile.country_code.to_string())
        });
    let manual_country = request
        .region
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .and_then(|value| {
            detect_profile_from_country_code(value).map(|profile| profile.country_code.to_string())
        });

    let resolved_country = detected_country.or(manual_country);
    let Some(country_code) = resolved_country else {
        return Err(AppError::bad_request(
            "region_required",
            "Unable to detect a supported region automatically. Please provide one of the supported country codes.",
        ));
    };
    let data_region = country_code_to_data_region(&country_code)
        .expect("supported country must map to a region")
        .as_str()
        .to_string();

    let birthdate = nvbes_core::auth::parse_birthdate(request.birthdate.as_deref())?;

    let result = onboarding::register(
        &state.db,
        &state.redis,
        &state.config,
        register_input_from_request(
            request,
            country_code,
            data_region,
            birthdate,
            ip,
            crate::http::request::user_agent(&headers),
        ),
    )
    .await?;

    state.product_analytics.capture(
        ProductAnalyticsEvent::workspace_for_user(
            "auth.signup_completed",
            result.user.id,
            result.workspace.id,
        )
        .property(
            "country",
            result
                .user
                .region
                .clone()
                .unwrap_or_else(|| "unknown".to_string()),
        )
        .property("workspace_type", result.workspace.workspace_type.clone()),
    );
    state.product_analytics.capture(
        ProductAnalyticsEvent::workspace_for_user(
            "workspace.created",
            result.user.id,
            result.workspace.id,
        )
        .property("workspace_type", result.workspace.workspace_type.clone()),
    );

    Ok(Json(result))
}

#[derive(Serialize, ToSchema)]
pub(crate) struct RegionResponse {
    region: Option<String>,
}

#[utoipa::path(
    get,
    path = "/auth/region",
    tag = "auth",
    responses(
        (status = 200, description = "Detected region from request headers", body = RegionResponse),
    ),
)]
pub(crate) async fn region(headers: HeaderMap) -> Json<RegionResponse> {
    Json(RegionResponse {
        region: crate::http::request::region_from_headers(&headers),
    })
}

#[derive(Serialize, ToSchema)]
pub(crate) struct SupportedRegionResponse {
    country_code: &'static str,
    data_region: &'static str,
    legal_jurisdiction: &'static str,
    primary_timezone: &'static str,
    timezones: Vec<&'static str>,
    sub_region: Option<&'static str>,
    display_name: Option<&'static str>,
}

#[utoipa::path(
    get,
    path = "/auth/regions",
    tag = "auth",
    responses(
        (status = 200, description = "Supported region catalog", body = [SupportedRegionResponse]),
    ),
)]
pub(crate) async fn supported_regions() -> Json<Vec<SupportedRegionResponse>> {
    Json(
        supported_profiles()
            .iter()
            .map(|profile| SupportedRegionResponse {
                country_code: profile.country_code,
                data_region: profile.data_region.as_str(),
                legal_jurisdiction: profile.legal_jurisdiction.as_str(),
                primary_timezone: profile.primary_timezone.as_str(),
                timezones: profile.timezones.iter().map(|tz| tz.as_str()).collect(),
                sub_region: profile.sub_region,
                display_name: profile.display_name,
            })
            .collect(),
    )
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
) -> Result<Json<crate::domains::auth::types::VerifyEmailResult>, AppError> {
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

    state.product_analytics.capture(ProductAnalyticsEvent::user(
        "auth.email_verified",
        result.user.id,
    ));

    Ok(Json(result))
}
