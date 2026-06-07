use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::http::error::AppError;

use super::{CLOCK_SKEW_SECONDS, ClientAssertionClaims, MAX_ASSERTION_TTL_SECONDS};

pub(crate) fn validate_claims(
    claims: &ClientAssertionClaims,
    client_id: &str,
) -> Result<(), AppError> {
    if claims.iss != client_id || claims.sub != client_id {
        return Err(AppError::unauthorized(
            "invalid_client",
            "The client_assertion issuer and subject must match the authenticated client.",
        ));
    }

    let now = Utc::now().timestamp();
    if claims.exp > now + MAX_ASSERTION_TTL_SECONDS {
        return Err(AppError::unauthorized(
            "invalid_client",
            "The client_assertion expiration is too far in the future.",
        ));
    }

    if claims.iat.is_some_and(|iat| iat > now + CLOCK_SKEW_SECONDS) {
        return Err(AppError::unauthorized(
            "invalid_client",
            "The client_assertion was issued in the future.",
        ));
    }

    let jti = claims.jti.as_deref().map(str::trim).unwrap_or("");
    if jti.is_empty() {
        return Err(AppError::unauthorized(
            "invalid_client",
            "The client_assertion jti is required.",
        ));
    }

    Ok(())
}

pub(crate) async fn record_assertion_jti(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    client_id: &str,
    jti: Option<&str>,
    exp: i64,
) -> Result<(), AppError> {
    let jti = jti.unwrap_or_default().trim();
    let expires_at = DateTime::<Utc>::from_timestamp(exp, 0).ok_or_else(|| {
        AppError::unauthorized(
            "invalid_client",
            "The client_assertion expiration is invalid.",
        )
    })?;

    sqlx::query("DELETE FROM oauth_client_assertion_jtis WHERE expires_at < NOW()")
        .execute(db)
        .await?;

    let inserted = sqlx::query(
        r#"
        INSERT INTO oauth_client_assertion_jtis (tenant_id, client_id, jti, expires_at)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (client_id, jti) DO NOTHING
        RETURNING jti
        "#,
    )
    .bind(tenant_id)
    .bind(client_id)
    .bind(jti)
    .bind(expires_at)
    .fetch_optional(db)
    .await?;

    if inserted.is_none() {
        return Err(AppError::unauthorized(
            "invalid_client",
            "The client_assertion has already been used.",
        ));
    }

    Ok(())
}
