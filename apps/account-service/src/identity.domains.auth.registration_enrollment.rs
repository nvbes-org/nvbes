use chrono::{DateTime, Duration, Utc};
use nvbes_core::config::AppConfig;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::{db, generate_random_token, token_hash};
use crate::http::error::AppError;

pub struct NewRegistrationEnrollment {
    pub token: String,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
}

pub fn generate(ttl_hours: i64) -> NewRegistrationEnrollment {
    let token = generate_random_token();
    NewRegistrationEnrollment {
        token_hash: token_hash(&token),
        token,
        expires_at: Utc::now() + Duration::hours(ttl_hours),
    }
}

pub async fn issue(
    db: &PgPool,
    config: &AppConfig,
    principal_id: Uuid,
) -> Result<String, AppError> {
    let enrollment = generate(config.auth_verification_ttl_hours);
    db::registration_enrollment::insert(
        db,
        principal_id,
        &enrollment.token_hash,
        enrollment.expires_at,
    )
    .await?;
    Ok(enrollment.token)
}

pub async fn consume_tx(tx: &mut Transaction<'_, Postgres>, token: &str) -> Result<Uuid, AppError> {
    db::registration_enrollment::consume_tx(tx, &token_hash(token)).await
}

pub async fn consume_all_for_principal_tx(
    tx: &mut Transaction<'_, Postgres>,
    principal_id: Uuid,
) -> Result<(), AppError> {
    db::registration_enrollment::consume_all_for_principal_tx(tx, principal_id).await
}

pub async fn delete_all_for_principal_tx(
    tx: &mut Transaction<'_, Postgres>,
    principal_id: Uuid,
) -> Result<(), AppError> {
    db::registration_enrollment::delete_all_for_principal_tx(tx, principal_id).await
}

#[cfg(test)]
mod tests {
    use super::generate;

    #[test]
    fn generated_enrollment_is_opaque_and_only_persists_its_hash() {
        let enrollment = generate(24);

        assert!(!enrollment.token.is_empty());
        assert_ne!(enrollment.token, enrollment.token_hash);
        assert_eq!(enrollment.token_hash.len(), 64);
    }
}
