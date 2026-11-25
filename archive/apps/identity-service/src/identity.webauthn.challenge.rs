use sqlx::{Postgres, Transaction};
use uuid::Uuid;

/// Recheck the deadline after all preceding row-lock waits and verification.
/// Callers roll back every credential/session mutation when consumption fails.
pub(crate) async fn consume(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    principal: Uuid,
    purpose: &str,
) -> Result<bool, sqlx::Error> {
    Ok(sqlx::query("UPDATE identity_webauthn_challenges SET consumed_at=clock_timestamp(),principal_id=$2 WHERE id=$1 AND purpose=$3 AND (principal_id IS NULL OR principal_id=$2) AND consumed_at IS NULL AND expires_at>clock_timestamp()")
        .bind(id).bind(principal).bind(purpose).execute(&mut **tx).await?.rows_affected() == 1)
}
