use super::{ResourceError, VerifiedResourceProof};
use chrono::{DateTime, Utc};
use sqlx::PgPool;

pub(super) async fn consume(
    db: &PgPool,
    proof: &VerifiedResourceProof,
    token_exp: u64,
) -> Result<(), ResourceError> {
    let mut tx = db.begin().await.map_err(|_| ResourceError::Unavailable)?;
    // A fixed namespace and 64 shards bound contention and total retained rows.
    sqlx::query("SELECT pg_advisory_xact_lock(1749102672, $1)")
        .bind(i32::from(proof.bucket))
        .execute(&mut *tx)
        .await
        .map_err(|_| ResourceError::Unavailable)?;
    if proof.expires_at <= Utc::now() || token_exp <= Utc::now().timestamp() as u64 {
        return Err(ResourceError::Invalid);
    }
    let token_exp = DateTime::from_timestamp(
        i64::try_from(token_exp).map_err(|_| ResourceError::Invalid)?,
        0,
    )
    .ok_or(ResourceError::Invalid)?;
    sqlx::query(
        "DELETE FROM resource_dpop_replays WHERE bucket=$1 AND expires_at<=clock_timestamp()",
    )
    .bind(proof.bucket)
    .execute(&mut *tx)
    .await
    .map_err(|_| ResourceError::Unavailable)?;
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM resource_dpop_replays WHERE bucket=$1")
            .bind(proof.bucket)
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| ResourceError::Unavailable)?;
    if count >= 256 {
        return Err(ResourceError::Unavailable);
    }
    // Cleanup and admission must agree on PostgreSQL time. Otherwise a database
    // clock ahead of the API could erase a marker while that API still accepts it.
    let inserted = sqlx::query("INSERT INTO resource_dpop_replays(bucket,jkt_hash,jti_hash,expires_at) SELECT $1,$2,$3,$4 WHERE $4>clock_timestamp() AND $5>clock_timestamp() ON CONFLICT DO NOTHING")
        .bind(proof.bucket).bind(&proof.jkt_hash).bind(&proof.jti_hash).bind(proof.expires_at).bind(token_exp)
        .execute(&mut *tx).await.map_err(|_| ResourceError::Unavailable)?.rows_affected();
    if inserted != 1 {
        return Err(ResourceError::Invalid);
    }
    tx.commit().await.map_err(|_| ResourceError::Unavailable)
}
