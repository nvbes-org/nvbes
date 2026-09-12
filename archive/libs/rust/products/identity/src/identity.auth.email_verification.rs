use sqlx::PgPool;

use crate::{IdentityError, IdentityResult};

const CLEANUP_ADVISORY_LOCK_ID: i64 = 20260519;

pub async fn cleanup_expired_unverified_identities(
    db: &PgPool,
    ttl_days: i64,
) -> IdentityResult<u64> {
    let mut tx = db.begin().await.map_err(IdentityError::from)?;
    let locked: bool = sqlx::query_scalar("SELECT pg_try_advisory_xact_lock($1)")
        .bind(CLEANUP_ADVISORY_LOCK_ID)
        .fetch_one(&mut *tx)
        .await
        .map_err(IdentityError::from)?;

    if !locked {
        tx.rollback().await.map_err(IdentityError::from)?;
        return Ok(0);
    }

    let deleted = sqlx::query(
        r#"
        DELETE FROM principals p
        USING users u
        WHERE p.id = u.principal_id
          AND u.status = 'pending_verification'
          AND u.email_verified_at IS NULL
          AND u.created_at < NOW() - make_interval(days => $1::int)
        RETURNING p.id
        "#,
    )
    .bind(ttl_days)
    .fetch_all(&mut *tx)
    .await
    .map_err(IdentityError::from)?;

    tx.commit().await.map_err(IdentityError::from)?;
    Ok(deleted.len() as u64)
}
