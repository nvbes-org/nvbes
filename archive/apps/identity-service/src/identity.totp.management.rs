use super::TotpError;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(serde::Serialize, sqlx::FromRow)]
pub struct FactorSummary {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
}

/// Returns only active, owned metadata; pending secrets are never recoverable.
pub async fn list(db: &PgPool, token: &str) -> Result<Vec<FactorSummary>, TotpError> {
    let mut tx = db.begin().await?;
    let principal = crate::factor_management::owner(&mut tx, token, false)
        .await?
        .ok_or(TotpError::Invalid)?;
    let rows = sqlx::query_as("SELECT id,created_at FROM identity_auth_factors WHERE principal_id=$1 AND kind='totp' AND state='active' LIMIT 1")
        .bind(principal).fetch_all(&mut *tx).await?;
    tx.commit().await?;
    Ok(rows)
}

/// Revoking TOTP ends every session of the principal, including the requester.
pub async fn revoke(db: &PgPool, token: &str, id: Uuid) -> Result<(), TotpError> {
    let mut tx = db.begin().await?;
    let principal = crate::factor_management::owner(&mut tx, token, true)
        .await?
        .ok_or(TotpError::Invalid)?;
    let state: Option<String> = sqlx::query_scalar("SELECT state FROM identity_auth_factors WHERE id=$1 AND principal_id=$2 AND kind='totp' FOR UPDATE")
        .bind(id).bind(principal).fetch_optional(&mut *tx).await?;
    match state.as_deref() {
        Some("revoked") => {
            tx.commit().await?;
            return Ok(());
        }
        Some("active") => {}
        _ => return Err(TotpError::Invalid),
    }
    // Same principal lock as WebAuthn revocation: competing removals cannot
    // each treat the other's factor as their surviving alternative.
    let alternative: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM identity_webauthn_credentials WHERE principal_id=$1 AND revoked_at IS NULL AND passkey IS NOT NULL)")
        .bind(principal).fetch_one(&mut *tx).await?;
    if !alternative {
        return Err(TotpError::LastFactor);
    }
    sqlx::query("UPDATE identity_auth_factors SET state='revoked',revoked_at=clock_timestamp(),secret_ciphertext=''::bytea,enrollment_session_id=NULL,enrollment_expires_at=NULL WHERE id=$1")
        .bind(id).execute(&mut *tx).await?;
    sqlx::query("UPDATE identity_sessions SET revoked_at=clock_timestamp() WHERE principal_id=$1 AND revoked_at IS NULL")
        .bind(principal).execute(&mut *tx).await?;
    crate::auth::audit(&mut tx, principal, "identity.mfa_totp_revoked").await?;
    tx.commit().await?;
    Ok(())
}

#[cfg(all(test, feature = "database-tests"))]
#[path = "identity.totp.management.tests.rs"]
mod tests;
