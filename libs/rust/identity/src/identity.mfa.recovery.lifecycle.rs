use super::{RecoveryError, event, owner};
use chrono::{DateTime, Utc};
use sqlx::PgPool;

/// Reading recovery state never extends its lifetime or grants normal authentication.
pub async fn expires_at(db: &PgPool, token: &str) -> Result<DateTime<Utc>, RecoveryError> {
    let mut tx = db.begin().await?;
    let (id, _) = owner(&mut tx, token).await?;
    let expiry =
        sqlx::query_scalar("SELECT expires_at FROM identity_mfa_recovery_sessions WHERE id=$1")
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
    tx.commit().await?;
    Ok(expiry)
}

/// Keep the consumed code and revoked sessions unchanged; cascade removes the ceremony.
pub async fn cancel(db: &PgPool, token: &str) -> Result<(), RecoveryError> {
    let mut tx = db.begin().await?;
    let (id, principal) = owner(&mut tx, token).await?;
    sqlx::query("DELETE FROM identity_mfa_recovery_sessions WHERE id=$1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    event(&mut tx, principal, "identity.mfa_recovery_cancelled").await?;
    tx.commit().await?;
    Ok(())
}
