use axum::{Json, extract::State, http::HeaderMap};

use crate::{app::AppState, error::AppError, registration_models::RegistrationProjection};

pub async fn project_registration(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(mut projection): Json<RegistrationProjection>,
) -> Result<Json<RegistrationProjectionResult>, AppError> {
    authorize(&headers, &state.config.provisioning_token)?;
    projection.validate()?;
    let applied = crate::registration_db::apply(&state.db, projection).await?;
    Ok(Json(RegistrationProjectionResult { applied }))
}

#[derive(serde::Serialize)]
pub struct RegistrationProjectionResult {
    applied: bool,
}

fn authorize(headers: &HeaderMap, expected: &str) -> Result<(), AppError> {
    let provided = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .unwrap_or_default();
    if !constant_time_eq(provided.as_bytes(), expected.as_bytes()) {
        return Err(AppError::unauthorized(
            "invalid_internal_token",
            "A valid Account provisioning token is required.",
        ));
    }
    Ok(())
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}

#[cfg(test)]
mod tests {
    use axum::http::{HeaderMap, HeaderValue, header};

    use super::authorize;

    #[test]
    fn internal_token_is_exact_and_never_accepted_by_prefix() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer secret-value"),
        );
        assert!(authorize(&headers, "secret-value").is_ok());
        assert!(authorize(&headers, "secret-value-extra").is_err());
    }
}
