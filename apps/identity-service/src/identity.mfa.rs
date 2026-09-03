use chrono::{DateTime, Duration, Utc};
use nvbes_core::mfa::{
    TOTP_WINDOW, current_counter, generate_totp_code, generate_totp_secret, verify_totp_code,
};
use serde::Serialize;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    auth::{audit, authenticate, create_synthetic_identity, hash_token},
    mfa_crypto::MfaCrypto,
};

const STEP_UP_TTL_MINUTES: i64 = 10;
type StoredFactor = (Uuid, Vec<u8>, Vec<u8>, i16, Option<i64>);

#[derive(Debug, Serialize)]
pub struct MfaSyntheticSmokeResult {
    pub principal_id: Uuid,
    pub factor_encrypted: bool,
    pub enrollment_confirmed: bool,
    pub step_up_granted: bool,
    pub replay_rejected: bool,
    pub audit_events: i64,
}

pub async fn run_synthetic_smoke(
    db: &PgPool,
    crypto: &MfaCrypto,
    email: &str,
    password: &str,
) -> anyhow::Result<MfaSyntheticSmokeResult> {
    let principal_id = create_synthetic_identity(db, email, password).await?;
    let session_token = authenticate(db, email, password).await?;
    let secret = enroll_totp(db, crypto, principal_id).await?;
    let now = Utc::now();
    let code = generate_totp_code(&secret, current_counter(now));
    let step_up_expires_at = confirm_enrollment(db, crypto, &session_token, &code, now).await?;
    let replay_rejected = grant_step_up(db, crypto, &session_token, &code, now)
        .await
        .is_err();
    let state: (String, bool) = sqlx::query_as(
        "SELECT state, secret_ciphertext <> convert_to($2, 'UTF8') FROM identity_auth_factors WHERE principal_id = $1",
    )
    .bind(principal_id)
    .bind(&secret)
    .fetch_one(db)
    .await?;
    let audit_events: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM identity_audit_events WHERE principal_id = $1")
            .bind(principal_id)
            .fetch_one(db)
            .await?;

    Ok(MfaSyntheticSmokeResult {
        principal_id,
        factor_encrypted: state.1,
        enrollment_confirmed: state.0 == "active",
        step_up_granted: step_up_expires_at > now,
        replay_rejected,
        audit_events,
    })
}

async fn enroll_totp(
    db: &PgPool,
    crypto: &MfaCrypto,
    principal_id: Uuid,
) -> anyhow::Result<String> {
    let factor_id = Uuid::new_v4();
    let secret = generate_totp_secret();
    let sealed = crypto.seal(factor_id, &secret)?;
    let mut tx = db.begin().await?;
    sqlx::query(
        "INSERT INTO identity_auth_factors (id, principal_id, kind, state, secret_ciphertext, secret_nonce, key_version) VALUES ($1, $2, 'totp', 'pending', $3, $4, $5)",
    )
    .bind(factor_id)
    .bind(principal_id)
    .bind(sealed.ciphertext)
    .bind(sealed.nonce.as_slice())
    .bind(sealed.key_version)
    .execute(&mut *tx)
    .await?;
    audit(
        &mut tx,
        principal_id,
        "identity.mfa_totp_enrollment_started",
    )
    .await?;
    tx.commit().await?;
    Ok(secret)
}

async fn confirm_enrollment(
    db: &PgPool,
    crypto: &MfaCrypto,
    session_token: &str,
    code: &str,
    now: DateTime<Utc>,
) -> anyhow::Result<DateTime<Utc>> {
    let mut tx = db.begin().await?;
    let session = active_session(&mut tx, session_token, now).await?;
    let factor = factor_for_update(&mut tx, session.1, "pending").await?;
    let counter = verified_counter(crypto, &factor, code, now)?;
    let expires_at = now + Duration::minutes(STEP_UP_TTL_MINUTES);
    sqlx::query(
        "UPDATE identity_auth_factors SET state = 'active', last_accepted_counter = $1 WHERE id = $2",
    )
    .bind(i64::try_from(counter)?)
    .bind(factor.0)
    .execute(&mut *tx)
    .await?;
    persist_session_grant(&mut tx, session.0, session.1, expires_at).await?;
    audit(&mut tx, session.1, "identity.mfa_totp_enrolled").await?;
    tx.commit().await?;
    Ok(expires_at)
}

async fn grant_step_up(
    db: &PgPool,
    crypto: &MfaCrypto,
    session_token: &str,
    code: &str,
    now: DateTime<Utc>,
) -> anyhow::Result<DateTime<Utc>> {
    let mut tx = db.begin().await?;
    let session = active_session(&mut tx, session_token, now).await?;
    let factor = factor_for_update(&mut tx, session.1, "active").await?;
    let counter = verified_counter(crypto, &factor, code, now)?;
    if factor.4.is_some_and(|last| counter <= last as u64) {
        anyhow::bail!("MFA code replay rejected");
    }
    sqlx::query("UPDATE identity_auth_factors SET last_accepted_counter = $1 WHERE id = $2")
        .bind(i64::try_from(counter)?)
        .bind(factor.0)
        .execute(&mut *tx)
        .await?;
    let expires_at = now + Duration::minutes(STEP_UP_TTL_MINUTES);
    persist_session_grant(&mut tx, session.0, session.1, expires_at).await?;
    tx.commit().await?;
    Ok(expires_at)
}

fn verified_counter(
    crypto: &MfaCrypto,
    factor: &StoredFactor,
    code: &str,
    now: DateTime<Utc>,
) -> anyhow::Result<u64> {
    let secret = crypto.open(factor.0, factor.3, &factor.1, &factor.2)?;
    verify_totp_code(&secret, code, now, TOTP_WINDOW)
        .ok_or_else(|| anyhow::anyhow!("MFA verification failed"))
}

async fn active_session(
    tx: &mut Transaction<'_, Postgres>,
    session_token: &str,
    now: DateTime<Utc>,
) -> anyhow::Result<(Uuid, Uuid)> {
    Ok(sqlx::query_as(
        "SELECT id, principal_id FROM identity_sessions WHERE token_hash = $1 AND revoked_at IS NULL AND expires_at > $2 FOR UPDATE",
    )
    .bind(hash_token(session_token))
    .bind(now)
    .fetch_one(&mut **tx)
    .await?)
}

async fn factor_for_update(
    tx: &mut Transaction<'_, Postgres>,
    principal_id: Uuid,
    state: &str,
) -> anyhow::Result<StoredFactor> {
    Ok(sqlx::query_as(
        "SELECT id, secret_ciphertext, secret_nonce, key_version, last_accepted_counter FROM identity_auth_factors WHERE principal_id = $1 AND kind = 'totp' AND state = $2 FOR UPDATE",
    )
    .bind(principal_id)
    .bind(state)
    .fetch_one(&mut **tx)
    .await?)
}

async fn persist_session_grant(
    tx: &mut Transaction<'_, Postgres>,
    session_id: Uuid,
    principal_id: Uuid,
    expires_at: DateTime<Utc>,
) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE identity_sessions SET step_up_expires_at = $1,step_up_method='totp' WHERE id = $2",
    )
    .bind(expires_at)
    .bind(session_id)
    .execute(&mut **tx)
    .await?;
    audit(tx, principal_id, "identity.step_up_granted").await?;
    Ok(())
}
