//! Internal password recovery used by the synthetic runtime diagnostics.
use crate::auth::{
    RecoveryNotification, audit, hash_token, normalize_email, random_token, validate_password,
};
use chrono::{Duration, Utc};
use nvbes_core::auth::hash_password;
use sqlx::PgPool;
use uuid::Uuid;

const RECOVERY_TTL_MINUTES: i64 = 15;

pub(super) async fn request_recovery(
    db: &PgPool,
    email: &str,
) -> anyhow::Result<RecoveryNotification> {
    let email = normalize_email(email)?;
    let principal_id: Uuid = sqlx::query_scalar(
        "SELECT principal_id FROM identity_login_identifiers WHERE kind = 'email' AND normalized_value = $1 AND verified_at IS NOT NULL",
    )
    .bind(&email)
    .fetch_one(db)
    .await?;
    let token = random_token();
    let challenge_id = Uuid::new_v4();
    let expires_at = Utc::now() + Duration::minutes(RECOVERY_TTL_MINUTES);
    let mut tx = db.begin().await?;
    sqlx::query(
        "INSERT INTO identity_recovery_challenges (id, principal_id, token_hash, expires_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(challenge_id)
    .bind(principal_id)
    .bind(hash_token(&token))
    .bind(expires_at)
    .execute(&mut *tx)
    .await?;
    audit(&mut tx, principal_id, "identity.recovery_requested").await?;
    tx.commit().await?;
    Ok(RecoveryNotification {
        challenge_id,
        principal_id,
        email,
        token,
        expires_at,
    })
}

pub(super) async fn reset_password(db: &PgPool, token: &str, password: &str) -> anyhow::Result<()> {
    validate_password(password)?;
    let password_hash =
        hash_password(password).map_err(|_| anyhow::anyhow!("password hashing failed"))?;
    let mut tx = db.begin().await?;
    let principal_id: Uuid = sqlx::query_scalar(
        "SELECT principal_id FROM identity_recovery_challenges WHERE token_hash = $1 AND consumed_at IS NULL AND expires_at > clock_timestamp() FOR UPDATE",
    )
    .bind(hash_token(token))
    .fetch_one(&mut *tx)
    .await?;
    sqlx::query("UPDATE identity_password_credentials SET password_hash = $1, changed_at = clock_timestamp(), compromised_at = NULL WHERE principal_id = $2")
        .bind(password_hash)
        .bind(principal_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE identity_recovery_challenges SET consumed_at = clock_timestamp() WHERE token_hash = $1")
        .bind(hash_token(token))
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE identity_sessions SET revoked_at = clock_timestamp() WHERE principal_id = $1 AND revoked_at IS NULL")
        .bind(principal_id)
        .execute(&mut *tx)
        .await?;
    audit(&mut tx, principal_id, "identity.password_recovered").await?;
    tx.commit().await?;
    Ok(())
}

pub(super) fn validate_password_pair(initial: &str, recovered: &str) -> anyhow::Result<()> {
    validate_password(initial)?;
    validate_password(recovered)?;
    if initial == recovered {
        anyhow::bail!("recovered password must differ from the initial password");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_password_pair;
    #[test]
    fn recovery_requires_a_distinct_strong_password() {
        assert!(validate_password_pair("long-password-one", "long-password-two").is_ok());
        assert!(validate_password_pair("same-password", "same-password").is_err());
        assert!(validate_password_pair("short", "long-password-two").is_err());
    }
}
