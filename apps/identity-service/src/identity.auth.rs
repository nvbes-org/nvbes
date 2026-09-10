use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};

use chrono::{DateTime, Duration, Utc};
use nvbes_core::auth::{dummy_verify_password, hash_password, verify_and_check_rehash};
use rand::RngCore;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Transaction};
use tokio::sync::Semaphore;
use uuid::Uuid;

const SESSION_TTL_HOURS: i64 = 1;
const PASSWORD_VERIFY_CONCURRENCY: usize = 2;

static PASSWORD_VERIFY_PERMITS: Semaphore = Semaphore::const_new(PASSWORD_VERIFY_CONCURRENCY);

#[derive(Debug, Clone)]
pub struct RecoveryNotification {
    pub challenge_id: Uuid,
    pub principal_id: Uuid,
    pub email: String,
    pub token: String,
    pub expires_at: DateTime<Utc>,
}

pub(super) async fn create_synthetic_identity(
    db: &PgPool,
    email: &str,
    password: &str,
) -> anyhow::Result<Uuid> {
    let mut tx = db.begin().await?;
    let principal_id =
        create_active_human(&mut tx, email, password, "identity.synthetic_created").await?;
    tx.commit().await?;
    Ok(principal_id)
}

pub(super) async fn create_active_human(
    tx: &mut Transaction<'_, Postgres>,
    email: &str,
    password: &str,
    audit_event: &str,
) -> anyhow::Result<Uuid> {
    let email = normalize_email(email)?;
    validate_password(password)?;
    let password_hash =
        hash_password(password).map_err(|_| anyhow::anyhow!("password hashing failed"))?;
    let principal_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO identity_principals (id, kind, status) VALUES ($1, 'human', 'active')",
    )
    .bind(principal_id)
    .execute(&mut **tx)
    .await?;
    sqlx::query(
        "INSERT INTO identity_login_identifiers (id, principal_id, kind, normalized_value, verified_at) VALUES ($1, $2, 'email', $3, clock_timestamp())",
    )
    .bind(Uuid::new_v4())
    .bind(principal_id)
    .bind(&email)
    .execute(&mut **tx)
    .await?;
    sqlx::query(
        "INSERT INTO identity_password_credentials (principal_id, password_hash) VALUES ($1, $2)",
    )
    .bind(principal_id)
    .bind(password_hash)
    .execute(&mut **tx)
    .await?;
    audit(tx, principal_id, audit_event).await?;
    outbox(tx, principal_id, "identity.principal.created.v1").await?;
    Ok(principal_id)
}

pub(crate) async fn authenticate(
    db: &PgPool,
    email: &str,
    password: &str,
) -> anyhow::Result<String> {
    let verified = verify_credentials(db, email, password).await?;
    let mut tx = db.begin().await?;
    let token = create_verified_session(&mut tx, verified).await?;
    tx.commit().await?;
    Ok(token)
}

/// Ephemeral evidence; no raw password, serialization, clone or caller-settable fields.
pub(crate) struct VerifiedPassword {
    principal_id: Uuid,
    password_hash: String,
    upgraded_hash: Option<String>,
    email: String,
}

pub(crate) async fn verify_credentials(
    db: &PgPool,
    email: &str,
    password: &str,
) -> anyhow::Result<VerifiedPassword> {
    let email = normalize_email(email)?;
    let credential = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT p.id, c.password_hash FROM identity_principals p JOIN identity_login_identifiers i ON i.principal_id = p.id JOIN identity_password_credentials c ON c.principal_id = p.id WHERE i.kind = 'email' AND i.normalized_value = $1 AND p.status = 'active'",
    )
    .bind(&email)
    .fetch_optional(db)
    .await?;
    let Some((principal_id, password_hash)) = credential else {
        verify_dummy_password(password).await?;
        anyhow::bail!("authentication failed");
    };
    let (password_valid, needs_rehash) = verify_stored_password(password, &password_hash).await?;
    if !password_valid {
        anyhow::bail!("authentication failed");
    }
    let upgraded_hash = if needs_rehash {
        Some(hash_current_password(password).await?)
    } else {
        None
    };
    Ok(VerifiedPassword {
        principal_id,
        password_hash,
        upgraded_hash,
        email,
    })
}

pub(crate) async fn create_verified_session(
    tx: &mut Transaction<'_, Postgres>,
    verified: VerifiedPassword,
) -> anyhow::Result<String> {
    let VerifiedPassword {
        principal_id,
        password_hash,
        upgraded_hash,
        email,
    } = verified;
    let token = random_token();
    let unchanged = sqlx::query_scalar::<_, bool>(
        "SELECT p.status = 'active' AND c.password_hash = $2 FROM identity_principals p JOIN identity_password_credentials c ON c.principal_id = p.id JOIN identity_login_identifiers i ON i.principal_id=p.id WHERE p.id = $1 AND i.kind='email' AND i.normalized_value=$3 FOR UPDATE OF p,c,i",
    )
    .bind(principal_id)
    .bind(&password_hash)
    .bind(email)
    .fetch_optional(&mut **tx)
    .await?
    .unwrap_or(false);
    if !unchanged {
        anyhow::bail!("authentication failed");
    }
    if let Some(upgraded_hash) = upgraded_hash {
        sqlx::query(
            "UPDATE identity_password_credentials SET password_hash=$1,changed_at=clock_timestamp() WHERE principal_id=$2 AND password_hash=$3",
        )
        .bind(upgraded_hash)
        .bind(principal_id)
        .bind(&password_hash)
        .execute(&mut **tx)
        .await?;
    }
    sqlx::query(
        "INSERT INTO identity_sessions (id, principal_id, token_hash, expires_at, authenticated_at, primary_amr) VALUES ($1, $2, $3, $4, clock_timestamp(), 'pwd')",
    )
    .bind(Uuid::new_v4())
    .bind(principal_id)
    .bind(hash_token(&token))
    .bind(Utc::now() + Duration::hours(SESSION_TTL_HOURS))
    .execute(&mut **tx)
    .await?;
    audit(tx, principal_id, "identity.authenticated").await?;
    Ok(token)
}

async fn verify_stored_password(
    password: &str,
    password_hash: &str,
) -> anyhow::Result<(bool, bool)> {
    let permit = PASSWORD_VERIFY_PERMITS
        .acquire()
        .await
        .map_err(|_| anyhow::anyhow!("password verification unavailable"))?;
    let password = password.to_owned();
    let password_hash = password_hash.to_owned();
    let verified = tokio::task::spawn_blocking(move || {
        verify_and_check_rehash(&password, &password_hash, None)
    })
    .await
    .map_err(|_| anyhow::anyhow!("password verification unavailable"))?
    .map_err(|_| anyhow::anyhow!("password verification failed"))?;
    drop(permit);
    Ok(verified)
}

async fn hash_current_password(password: &str) -> anyhow::Result<String> {
    let permit = PASSWORD_VERIFY_PERMITS
        .acquire()
        .await
        .map_err(|_| anyhow::anyhow!("password hashing unavailable"))?;
    let password = password.to_owned();
    let hash = tokio::task::spawn_blocking(move || hash_password(&password))
        .await
        .map_err(|_| anyhow::anyhow!("password hashing unavailable"))?
        .map_err(|_| anyhow::anyhow!("password hashing failed"))?;
    drop(permit);
    Ok(hash)
}

async fn verify_dummy_password(password: &str) -> anyhow::Result<()> {
    let permit = PASSWORD_VERIFY_PERMITS
        .acquire()
        .await
        .map_err(|_| anyhow::anyhow!("password verification unavailable"))?;
    let password = password.to_owned();
    tokio::task::spawn_blocking(move || dummy_verify_password(&password, None))
        .await
        .map_err(|_| anyhow::anyhow!("password verification unavailable"))?;
    drop(permit);
    Ok(())
}

pub(super) async fn audit(
    tx: &mut Transaction<'_, Postgres>,
    principal_id: Uuid,
    event_type: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO identity_audit_events (id, principal_id, actor_principal_id, event_type, correlation_id) VALUES ($1, $2, $2, $3, $4)")
        .bind(Uuid::new_v4())
        .bind(principal_id)
        .bind(event_type)
        .bind(Uuid::new_v4())
        .execute(&mut **tx)
        .await?;
    Ok(())
}

async fn outbox(
    tx: &mut Transaction<'_, Postgres>,
    principal_id: Uuid,
    event_type: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO identity_outbox (id, event_type, aggregate_id, payload) VALUES ($1, $2, $3, $4)")
        .bind(Uuid::new_v4())
        .bind(event_type)
        .bind(principal_id)
        .bind(serde_json::json!({ "principal_id": principal_id }))
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub(super) fn random_token() -> String {
    let mut bytes = [0_u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

pub(super) fn hash_token(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

pub(super) fn normalize_email(email: &str) -> anyhow::Result<String> {
    let email = email.trim().to_ascii_lowercase();
    let valid = email.len() <= 320
        && email
            .split_once('@')
            .is_some_and(|(local, domain)| !local.is_empty() && domain.contains('.'));
    valid
        .then_some(email)
        .ok_or_else(|| anyhow::anyhow!("synthetic email is invalid"))
}

pub(super) fn validate_password(password: &str) -> anyhow::Result<()> {
    if !(12..=1024).contains(&password.len()) {
        anyhow::bail!("synthetic password must contain between 12 and 1024 bytes");
    }
    Ok(())
}

#[cfg(test)]
#[path = "identity.auth.tests.rs"]
mod tests;
