use crate::oauth::store::{hash, random_secret};
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

#[path = "identity.mfa.recovery.lifecycle.rs"]
pub mod lifecycle;
#[path = "identity.mfa.recovery.registration.rs"]
pub mod registration;

#[derive(Debug, thiserror::Error)]
pub enum RecoveryError {
    #[error("invalid recovery request or authentication")]
    Invalid,
    #[error("recovery persistence unavailable")]
    Database(#[from] sqlx::Error),
    #[error("invalid stored recovery ceremony")]
    Serialization(#[from] serde_json::Error),
}

// Secret-bearing responses deliberately do not implement Debug.
pub struct RecoveryCodes {
    pub codes: Vec<String>,
}
pub struct RecoverySession {
    pub token: String,
    pub expires_at: DateTime<Utc>,
}

fn code_hash(principal: Uuid, code: &str) -> Vec<u8> {
    let mut digest = Sha256::new();
    digest.update(b"nvbes:mfa-recovery:v1:");
    digest.update(principal.as_bytes());
    digest.update(code.as_bytes());
    digest.finalize().to_vec()
}

/// Strong recent authentication is mandatory; generation atomically replaces the batch.
pub async fn generate(db: &PgPool, token: &str) -> Result<RecoveryCodes, RecoveryError> {
    let mut tx = db.begin().await?;
    let principal = crate::factor_management::owner(&mut tx, token, true)
        .await?
        .ok_or(RecoveryError::Invalid)?;
    let active: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM identity_webauthn_credentials WHERE principal_id=$1 AND revoked_at IS NULL AND passkey IS NOT NULL) OR EXISTS(SELECT 1 FROM identity_auth_factors WHERE principal_id=$1 AND state='active')")
        .bind(principal).fetch_one(&mut *tx).await?;
    if !active {
        return Err(RecoveryError::Invalid);
    }
    sqlx::query("DELETE FROM identity_mfa_recovery_codes WHERE principal_id=$1")
        .bind(principal)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM identity_mfa_recovery_sessions WHERE principal_id=$1")
        .bind(principal)
        .execute(&mut *tx)
        .await?;
    let mut codes = Vec::with_capacity(10);
    for _ in 0..10 {
        let code = format!("nvr1_{}", random_secret());
        sqlx::query(
            "INSERT INTO identity_mfa_recovery_codes(principal_id,code_hash) VALUES($1,$2)",
        )
        .bind(principal)
        .bind(code_hash(principal, &code))
        .execute(&mut *tx)
        .await?;
        codes.push(code);
    }
    event(&mut tx, principal, "identity.mfa_recovery_codes_generated").await?;
    tx.commit().await?;
    Ok(RecoveryCodes { codes })
}

/// A recent normal login plus a saved code yields only a recovery token.
/// Every ordinary session is revoked before the recovery token can leave the transaction.
pub async fn redeem(
    db: &PgPool,
    session_token: &str,
    code: &str,
) -> Result<RecoverySession, RecoveryError> {
    if code.len() != 48
        || !code.starts_with("nvr1_")
        || !code[5..]
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(RecoveryError::Invalid);
    }
    let mut tx = db.begin().await?;
    let principal = crate::authentication_session::load(&mut tx, session_token)
        .await?
        .filter(|session| session.recent_primary)
        .map(|session| session.principal)
        .ok_or(RecoveryError::Invalid)?;
    let consumed=sqlx::query("UPDATE identity_mfa_recovery_codes SET consumed_at=clock_timestamp() WHERE principal_id=$1 AND code_hash=$2 AND consumed_at IS NULL")
        .bind(principal).bind(code_hash(principal,code)).execute(&mut *tx).await?.rows_affected();
    if consumed != 1 {
        return Err(RecoveryError::Invalid);
    }
    sqlx::query("DELETE FROM identity_mfa_recovery_sessions WHERE principal_id=$1")
        .bind(principal)
        .execute(&mut *tx)
        .await?;
    let token = random_secret();
    let expires_at=sqlx::query_scalar("INSERT INTO identity_mfa_recovery_sessions(id,principal_id,token_hash,expires_at) VALUES($1,$2,$3,clock_timestamp()+interval '5 minutes') RETURNING expires_at")
        .bind(Uuid::new_v4()).bind(principal).bind(hash(&token)).fetch_one(&mut *tx).await?;
    sqlx::query("UPDATE identity_sessions SET revoked_at=clock_timestamp() WHERE principal_id=$1 AND revoked_at IS NULL").bind(principal).execute(&mut *tx).await?;
    event(&mut tx, principal, "identity.mfa_recovery_started").await?;
    tx.commit().await?;
    Ok(RecoverySession { token, expires_at })
}

async fn owner(
    tx: &mut Transaction<'_, Postgres>,
    token: &str,
) -> Result<(Uuid, Uuid), RecoveryError> {
    let principal: Option<Uuid> = sqlx::query_scalar(
        "SELECT principal_id FROM identity_mfa_recovery_sessions WHERE token_hash=$1",
    )
    .bind(hash(token))
    .fetch_optional(&mut **tx)
    .await?;
    crate::session_locks::principal(tx, principal.ok_or(RecoveryError::Invalid)?).await?;
    sqlx::query_as("SELECT r.id,r.principal_id FROM identity_mfa_recovery_sessions r JOIN identity_principals p ON p.id=r.principal_id WHERE r.token_hash=$1 AND r.expires_at>clock_timestamp() AND p.status='active' FOR UPDATE OF r,p")
        .bind(hash(token)).fetch_optional(&mut **tx).await?.ok_or(RecoveryError::Invalid)
}

async fn event(
    tx: &mut Transaction<'_, Postgres>,
    principal: Uuid,
    name: &str,
) -> Result<(), sqlx::Error> {
    crate::auth::audit(tx, principal, name).await?;
    let event_id = Uuid::new_v4();
    let occurred_at = sqlx::query_scalar(
        "INSERT INTO identity_outbox(id,event_type,aggregate_id,payload) VALUES($1,$2,$3,$4) RETURNING occurred_at",
    )
    .bind(event_id)
    .bind(name)
    .bind(principal)
    .bind(serde_json::json!({"principal_id":principal}))
    .fetch_one(&mut **tx)
    .await?;
    crate::notification_queue::enqueue(tx, event_id, principal, name, occurred_at).await?;
    Ok(())
}

#[cfg(all(test, feature = "database-tests"))]
#[path = "identity.mfa.recovery.tests.rs"]
mod tests;
