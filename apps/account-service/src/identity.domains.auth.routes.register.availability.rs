use axum::{Json, extract::State, http::HeaderMap};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use utoipa::ToSchema;

use crate::{app::AppState, http::error::AppError};
use nvbes_core::http::error::ErrorEnvelope;

#[derive(Debug, Clone, Copy, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RegistrationAvailabilityField {
    Email,
    Username,
}

#[derive(Deserialize, ToSchema)]
pub(crate) struct RegistrationAvailabilityRequest {
    field: RegistrationAvailabilityField,
    value: String,
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
    let normalized_value = normalize_availability_value(request.field, &request.value)?;
    let scoped_key =
        crate::domains::auth::token_hash(&format!("{:?}:{normalized_value}", request.field));

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

    let exists = match request.field {
        RegistrationAvailabilityField::Email => {
            crate::domains::auth::db::registration_email_exists(&state.db, &normalized_value)
                .await?
        }
        RegistrationAvailabilityField::Username => {
            crate::domains::auth::db::registration_username_exists(&state.db, &normalized_value)
                .await?
        }
    };

    Ok(Json(RegistrationAvailabilityResponse {
        available: !exists,
    }))
}

fn normalize_availability_value(
    field: RegistrationAvailabilityField,
    value: &str,
) -> Result<String, AppError> {
    match field {
        RegistrationAvailabilityField::Email => {
            let email = crate::domains::auth::normalize_email(value);
            crate::domains::auth::validate_email(&email)?;
            Ok(email)
        }
        RegistrationAvailabilityField::Username => {
            crate::domains::auth::username::normalize_username(value)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{RegistrationAvailabilityField, normalize_availability_value};

    #[test]
    fn availability_values_use_registration_normalization() {
        assert_eq!(
            normalize_availability_value(
                RegistrationAvailabilityField::Email,
                " User@Example.COM "
            )
            .unwrap(),
            "user@example.com"
        );
        assert_eq!(
            normalize_availability_value(RegistrationAvailabilityField::Username, " Ada ").unwrap(),
            "Ada"
        );
    }

    #[test]
    fn availability_rejects_invalid_values_before_database_lookup() {
        assert!(
            normalize_availability_value(RegistrationAvailabilityField::Email, "invalid").is_err()
        );
        assert!(
            normalize_availability_value(RegistrationAvailabilityField::Username, "   ").is_err()
        );
    }
}
