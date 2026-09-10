//! Internal password recovery used by the synthetic runtime diagnostics.
use crate::auth::{
    RecoveryNotification, audit, hash_current_password, hash_token, normalize_email, random_token,
    validate_password, verify_stored_password,
};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

pub(super) async fn request_recovery(
    db: &PgPool,
    email: &str,
) -> anyhow::Result<RecoveryNotification> {
    let mut tx = db.begin().await?;
    let recovery = issue(&mut tx, email).await?;
    tx.commit().await?;
    Ok(recovery)
}

pub(crate) async fn issue(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    email: &str,
) -> anyhow::Result<RecoveryNotification> {
    let email = normalize_email(email)?;
    let principal_id: Uuid = sqlx::query_scalar(
        "SELECT i.principal_id FROM identity_login_identifiers i JOIN identity_principals p ON p.id=i.principal_id JOIN identity_password_credentials c ON c.principal_id=p.id WHERE i.kind='email' AND i.normalized_value=$1 AND i.verified_at IS NOT NULL AND p.status='active' AND p.kind='human'",
    )
    .bind(&email)
    .fetch_one(&mut **tx)
    .await?;
    let token = random_token();
    let challenge_id = Uuid::new_v4();
    crate::session_locks::principal(tx, principal_id).await?;
    // Revalidate the recipient and account after acquiring the shared lifecycle lock.
    let eligible: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM identity_principals p JOIN identity_login_identifiers i ON i.principal_id=p.id JOIN identity_password_credentials c ON c.principal_id=p.id WHERE p.id=$1 AND p.status='active' AND p.kind='human' AND i.kind='email' AND i.normalized_value=$2 AND i.verified_at IS NOT NULL)",
    )
    .bind(principal_id)
    .bind(&email)
    .fetch_one(&mut **tx)
    .await?;
    anyhow::ensure!(eligible, "recovery unavailable");
    let expires_at: DateTime<Utc> = sqlx::query_scalar(
        "INSERT INTO identity_recovery_challenges (id, principal_id, token_hash, expires_at) VALUES ($1, $2, $3, clock_timestamp()+interval '15 minutes') RETURNING expires_at",
    )
    .bind(challenge_id)
    .bind(principal_id)
    .bind(hash_token(&token))
    .fetch_one(&mut **tx)
    .await?;
    audit(tx, principal_id, "identity.recovery_requested").await?;
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
    anyhow::ensure!(
        token.len() == 43
            && token
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-'),
        "recovery unavailable"
    );
    let token_hash = hash_token(token);
    // Read a candidate before expensive hashing, without holding database locks.
    // Neither this read nor the password comparison authorizes the mutation.
    let (principal_id, previous_hash): (Uuid, String) = sqlx::query_as(
        "SELECT r.principal_id,c.password_hash FROM identity_recovery_challenges r JOIN identity_principals p ON p.id=r.principal_id JOIN identity_password_credentials c ON c.principal_id=p.id WHERE r.token_hash=$1 AND r.consumed_at IS NULL AND r.expires_at>clock_timestamp() AND p.status='active' AND p.kind='human'",
    )
    .bind(&token_hash)
    .fetch_one(db)
    .await?;
    let (unchanged_password, _) = verify_stored_password(password, &previous_hash).await?;
    anyhow::ensure!(
        !unchanged_password,
        "recovered password must differ from the current password"
    );
    let password_hash = hash_current_password(password).await?;
    let mut tx = db.begin().await?;
    // All competing reset links use the same ordering as login and revocation.
    // Locking a challenge first would deadlock when invalidating sibling links.
    crate::session_locks::principal(&mut tx, principal_id).await?;
    let valid: bool = sqlx::query_scalar(
        "SELECT p.status='active' AND p.kind='human' AND c.password_hash=$3 AND r.consumed_at IS NULL AND r.expires_at>clock_timestamp() FROM identity_recovery_challenges r JOIN identity_principals p ON p.id=r.principal_id JOIN identity_password_credentials c ON c.principal_id=p.id WHERE r.token_hash=$1 AND r.principal_id=$2 FOR UPDATE OF r,c",
    )
    .bind(&token_hash)
    .bind(principal_id)
    .bind(previous_hash)
    .fetch_optional(&mut *tx)
    .await?
    .unwrap_or(false);
    anyhow::ensure!(valid, "recovery unavailable");
    sqlx::query("UPDATE identity_password_credentials SET password_hash = $1, changed_at = clock_timestamp(), compromised_at = NULL WHERE principal_id = $2")
        .bind(password_hash)
        .bind(principal_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE identity_recovery_challenges SET consumed_at = clock_timestamp() WHERE principal_id=$1 AND consumed_at IS NULL")
        .bind(principal_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE identity_sessions SET revoked_at = clock_timestamp() WHERE principal_id = $1 AND revoked_at IS NULL")
        .bind(principal_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE identity_recovery_deliveries SET state='cancelled',outcome='password_recovered',ciphertext=NULL,nonce=NULL,key_version=NULL,lease_token=NULL,lease_expires_at=NULL,settled_at=clock_timestamp() WHERE principal_id=$1 AND state IN ('pending','sending')")
        .bind(principal_id).execute(&mut *tx).await?;
    audit(&mut tx, principal_id, "identity.password_recovered").await?;
    tx.commit().await?;
    Ok(())
}

#[cfg(all(test, feature = "database-tests"))]
#[path = "identity.recovery.database.tests.rs"]
mod database_tests;

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
