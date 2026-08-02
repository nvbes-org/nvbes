use axum::{Json, extract::State, http::HeaderMap};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use utoipa::ToSchema;

use crate::{app::AppState, http::error::AppError};
use nvbes_core::http::error::ErrorEnvelope;

#[derive(Deserialize, ToSchema)]
pub(crate) struct RegistrationAvailabilityRequest {
    email: String,
}

#[derive(Serialize, ToSchema)]
pub(crate) struct RegistrationAvailabilityResponse {
    available: bool,
}

#[utoipa::path(
    post,
    path = "/auth/registration/availability",
    tag = "auth",
    request_body = RegistrationAvailabilityRequest,
    responses(
        (status = 200, description = "Registration identifier availability", body = RegistrationAvailabilityResponse),
        (status = 400, description = "Invalid registration identifier", body = ErrorEnvelope),
        (status = 429, description = "Rate limited", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn registration_availability(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<RegistrationAvailabilityRequest>,
) -> Result<Json<RegistrationAvailabilityResponse>, AppError> {
    let email = normalize_availability_email(&request.email)?;
    let scoped_key = crate::domains::auth::token_hash(&email);

    nvbes_core::limiter::check_dual_rate_limit(
        &state.redis,
        &headers,
        "auth_registration_availability",
        &scoped_key,
        40,
        12,
        Duration::from_secs(60),
    )
    .await?;

    let exists = crate::domains::auth::db::registration_email_exists(&state.db, &email).await?;

    Ok(Json(RegistrationAvailabilityResponse {
        available: !exists,
    }))
}

fn normalize_availability_email(value: &str) -> Result<String, AppError> {
    let email = crate::domains::auth::normalize_email(value);
    crate::domains::auth::validate_email(&email)?;
    Ok(email)
}

#[cfg(test)]
mod tests {
    use super::normalize_availability_email;

    #[test]
    fn availability_values_use_registration_normalization() {
        assert_eq!(
            normalize_availability_email(" User@Example.COM ").unwrap(),
            "user@example.com"
        );
    }

    #[test]
    fn availability_rejects_invalid_values_before_database_lookup() {
        assert!(normalize_availability_email("invalid").is_err());
    }
}
