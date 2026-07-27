use crate::domains::auth::{exposed_credentials, onboarding};
use crate::{app::AppState, http::error::AppError};
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use nvbes_core::http::error::ErrorEnvelope;
use nvbes_product_analytics::ProductAnalyticsEvent;
use nvbes_region::{
    country_code_to_data_region, detect_profile_from_country_code, is_country_allowed,
    supported_data_regions, supported_profiles,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use tracing::{info, warn};
use utoipa::ToSchema;

#[cfg(test)]
#[path = "identity.domains.auth.routes.register.tests.rs"]
mod tests;

const REGISTER_REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const SUPPORTED_REGIONS_CACHE_CONTROL: &str =
    "public, max-age=86400, stale-while-revalidate=604800";

static SUPPORTED_REGIONS_CACHE: OnceLock<SupportedRegionsCache> = OnceLock::new();

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
    firstname: String,
    lastname: String,
    username: String,
    birthdate: Option<String>,
    region: Option<String>,
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
    country_code: String,
    data_region: String,
    birthdate: Option<chrono::NaiveDate>,
    ip: Option<String>,
    user_agent: Option<String>,
) -> crate::domains::auth::types::RegisterInput {
    crate::domains::auth::types::RegisterInput {
        email: request.email,
        password: request.password,
        firstname: request.firstname.trim().to_string(),
        lastname: request.lastname.trim().to_string(),
        username: request.username,
        birthdate,
        region: Some(country_code),
        data_region: Some(data_region),
        ip,
        user_agent,
        legal_documents_accepted: request.legal_documents_accepted,
        marketing_emails_accepted: request.marketing_emails_accepted,
    }
}

fn validate_required_profile_name(value: &str, field: &str) -> Result<(), AppError> {
    if value.trim().is_empty() {
        return Err(AppError::bad_request(
            format!("{field}_required"),
            format!("{field} is required."),
        ));
    }

    Ok(())
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
    let started_at = Instant::now();
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
            Ok(result)
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

async fn register_inner(
    state: AppState,
    headers: HeaderMap,
    request: RegisterRequest,
) -> Result<Json<crate::domains::auth::types::RegisterResult>, AppError> {
    info!(
        has_manual_region = request
            .region
            .as_deref()
            .map(str::trim)
            .is_some_and(|value| !value.is_empty()),
        "auth_register_started"
    );

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
    info!(country = %country_code, "auth_register_region_resolved");
    let data_region = country_code_to_data_region(&country_code)
        .expect("supported country must map to a region")
        .as_str()
        .to_string();

    let birthdate = nvbes_core::auth::parse_birthdate(request.birthdate.as_deref())?;
    validate_required_profile_name(&request.firstname, "firstname")?;
    validate_required_profile_name(&request.lastname, "lastname")?;
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
            country_code,
            data_region,
            birthdate,
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
            .property(
                "country",
                result
                    .user
                    .region
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string()),
            ),
    );
    info!("auth_register_analytics_captured");

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

#[derive(Clone, Serialize, ToSchema)]
pub(crate) struct SupportedRegionResponse {
    country_code: &'static str,
    data_region: &'static str,
    legal_jurisdiction: &'static str,
    primary_timezone: &'static str,
    timezones: Vec<&'static str>,
    sub_region: Option<&'static str>,
    display_name: Option<&'static str>,
    hosting_strategy: &'static str,
    is_european_exclusive: bool,
}

struct SupportedRegionsCache {
    etag: String,
    body: Vec<SupportedRegionResponse>,
}

fn supported_region_catalog() -> Vec<SupportedRegionResponse> {
    let allowed_data_regions = supported_data_regions();
    let mut regions: Vec<_> = supported_profiles()
        .iter()
        .filter(|profile| {
            allowed_data_regions.contains(&profile.data_region)
                && is_country_allowed(profile.country_code)
        })
        .map(|profile| SupportedRegionResponse {
            country_code: profile.country_code,
            data_region: profile.data_region.as_str(),
            legal_jurisdiction: profile.legal_jurisdiction.as_str(),
            primary_timezone: profile.primary_timezone.as_str(),
            timezones: profile.timezones.iter().map(|tz| tz.as_str()).collect(),
            sub_region: profile.sub_region,
            display_name: profile.display_name,
            hosting_strategy: profile.data_region.hosting_strategy(),
            is_european_exclusive: profile.data_region.is_european_exclusive(),
        })
        .collect();

    regions.sort_by(|left, right| left.country_code.cmp(right.country_code));
    regions
}

fn supported_regions_cache() -> &'static SupportedRegionsCache {
    SUPPORTED_REGIONS_CACHE.get_or_init(|| {
        let body = supported_region_catalog();
        let json = serde_json::to_vec(&body).expect("supported regions catalog must serialize");
        let hash = Sha256::digest(&json);

        SupportedRegionsCache {
            etag: format!("\"regions-{}\"", hex::encode(hash)),
            body,
        }
    })
}

fn if_none_match_matches(headers: &HeaderMap, etag: &str) -> bool {
    headers
        .get(header::IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                .split(',')
                .map(str::trim)
                .any(|candidate| candidate == "*" || candidate == etag)
        })
}

fn insert_supported_regions_cache_headers(headers: &mut HeaderMap, etag: &str) {
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(SUPPORTED_REGIONS_CACHE_CONTROL),
    );
    headers.insert(
        header::ETAG,
        HeaderValue::from_str(etag).expect("supported regions ETag must be a valid header"),
    );
}

#[utoipa::path(
    get,
    path = "/auth/regions",
    tag = "auth",
    responses(
        (status = 200, description = "Supported region catalog", body = [SupportedRegionResponse]),
        (status = 304, description = "Supported region catalog not modified"),
    ),
)]
pub(crate) async fn supported_regions(headers: HeaderMap) -> Response {
    let cache = supported_regions_cache();

    if if_none_match_matches(&headers, &cache.etag) {
        let mut response = StatusCode::NOT_MODIFIED.into_response();
        insert_supported_regions_cache_headers(response.headers_mut(), &cache.etag);
        return response;
    }

    let mut response = Json(cache.body.clone()).into_response();
    insert_supported_regions_cache_headers(response.headers_mut(), &cache.etag);
    response
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

    let (distinct_id, session_id) = crate::http::request::product_analytics_correlation(&headers);
    state.product_analytics.capture(
        ProductAnalyticsEvent::user("auth.email_verified", result.user.id)
            .correlation(distinct_id, session_id),
    );

    Ok(Json(result))
}
