use crate::{auth::audit, mfa_crypto::MfaCrypto};
use chrono::{DateTime, Utc};
use nvbes_core::mfa::{TOTP_WINDOW, generate_totp_secret, provisioning_uri, verify_totp_code};
use sqlx::PgPool;
use uuid::Uuid;

#[path = "identity.totp.management.rs"]
pub mod management;

#[derive(Debug, thiserror::Error)]
pub enum TotpError {
    #[error("invalid TOTP ceremony or authentication")]
    Invalid,
    #[error("TOTP factor already active")]
    AlreadyActive,
    #[error("cannot revoke the last strong factor")]
    LastFactor,
    #[error("TOTP secret protection unavailable")]
    Crypto,
    #[error("TOTP persistence unavailable")]
    Database(#[from] sqlx::Error),
}

// Deliberately not Debug: this one-time response contains the provisioning secret.
#[derive(serde::Serialize)]
pub struct Enrollment {
    pub factor_id: Uuid,
    pub secret_base32: String,
    pub provisioning_uri: String,
    pub expires_at: DateTime<Utc>,
}

pub async fn start(db: &PgPool, crypto: &MfaCrypto, token: &str) -> Result<Enrollment, TotpError> {
    let mut tx = db.begin().await?;
    let (session, principal) = crate::enrollment_policy::owner(&mut tx, token)
        .await?
        .ok_or(TotpError::Invalid)?;
    let active: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM identity_auth_factors WHERE principal_id=$1 AND kind='totp' AND state='active')")
        .bind(principal).fetch_one(&mut *tx).await?;
    if active {
        return Err(TotpError::AlreadyActive);
    }
    let id = Uuid::new_v4();
    let secret = generate_totp_secret();
    let sealed = crypto.seal(id, &secret).map_err(|_| TotpError::Crypto)?;
    let expires = sqlx::query_scalar("INSERT INTO identity_auth_factors(id,principal_id,kind,state,secret_ciphertext,secret_nonce,key_version,enrollment_session_id,enrollment_expires_at) VALUES($1,$2,'totp','pending',$3,$4,$5,$6,clock_timestamp()+interval '5 minutes') ON CONFLICT(principal_id,kind) DO UPDATE SET id=EXCLUDED.id,state='pending',secret_ciphertext=EXCLUDED.secret_ciphertext,secret_nonce=EXCLUDED.secret_nonce,key_version=EXCLUDED.key_version,enrollment_session_id=EXCLUDED.enrollment_session_id,enrollment_expires_at=EXCLUDED.enrollment_expires_at,last_accepted_counter=NULL,revoked_at=NULL,created_at=clock_timestamp() WHERE identity_auth_factors.state<>'active' RETURNING enrollment_expires_at")
        .bind(id).bind(principal).bind(sealed.ciphertext).bind(sealed.nonce.as_slice())
        .bind(sealed.key_version).bind(session).fetch_one(&mut *tx).await?;
    audit(&mut tx, principal, "identity.mfa_totp_enrollment_started").await?;
    tx.commit().await?;
    Ok(Enrollment {
        factor_id: id,
        provisioning_uri: provisioning_uri("nvbes", &principal.to_string(), &secret),
        secret_base32: secret,
        expires_at: expires,
    })
}

pub async fn confirm(
    db: &PgPool,
    crypto: &MfaCrypto,
    token: &str,
    id: Uuid,
    code: &str,
) -> Result<DateTime<Utc>, TotpError> {
    if code.len() != 6 || !code.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(TotpError::Invalid);
    }
    let mut tx = db.begin().await?;
    let (session, principal) = crate::enrollment_policy::owner(&mut tx, token)
        .await?
        .ok_or(TotpError::Invalid)?;
    let factor: Option<(Vec<u8>, Vec<u8>, i16)> = sqlx::query_as("SELECT secret_ciphertext,secret_nonce,key_version FROM identity_auth_factors WHERE id=$1 AND principal_id=$2 AND enrollment_session_id=$3 AND kind='totp' AND state='pending' AND enrollment_expires_at>clock_timestamp() FOR UPDATE")
        .bind(id).bind(principal).bind(session).fetch_optional(&mut *tx).await?;
    let (ciphertext, nonce, version) = factor.ok_or(TotpError::Invalid)?;
    let secret = crypto
        .open(id, version, &ciphertext, &nonce)
        .map_err(|_| TotpError::Crypto)?;
    let now: DateTime<Utc> = sqlx::query_scalar("SELECT clock_timestamp()")
        .fetch_one(&mut *tx)
        .await?;
    let counter = verify_totp_code(&secret, code, now, TOTP_WINDOW).ok_or(TotpError::Invalid)?;
    let counter = i64::try_from(counter).map_err(|_| TotpError::Invalid)?;
    if crate::enrollment_policy::owner(&mut tx, token).await? != Some((session, principal)) {
        return Err(TotpError::Invalid);
    }
    let activated = sqlx::query("UPDATE identity_auth_factors SET state='active',last_accepted_counter=$2,enrollment_session_id=NULL,enrollment_expires_at=NULL WHERE id=$1 AND state='pending' AND enrollment_expires_at>clock_timestamp()")
        .bind(id).bind(counter).execute(&mut *tx).await?.rows_affected();
    if activated != 1 {
        return Err(TotpError::Invalid);
    }
    let expires = sqlx::query_scalar("UPDATE identity_sessions SET step_up_method='totp',step_up_at=clock_timestamp(),step_up_expires_at=LEAST(expires_at,clock_timestamp()+interval '10 minutes') WHERE id=$1 RETURNING step_up_expires_at")
        .bind(session).fetch_one(&mut *tx).await?;
    audit(&mut tx, principal, "identity.mfa_totp_enrolled").await?;
    audit(&mut tx, principal, "identity.step_up_granted").await?;
    tx.commit().await?;
    Ok(expires)
}

#[cfg(all(test, feature = "database-tests"))]
#[path = "identity.totp.tests.rs"]
mod tests;
