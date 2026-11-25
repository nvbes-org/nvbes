use sqlx::PgPool;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct CleanupResult {
    pub requests: u64,
    pub unexchanged_codes: u64,
    pub consents: u64,
}

/// At most 64 rows per table/call; SKIP LOCKED never waits on an interaction.
/// Codes referenced by grants remain available for valid replay detection.
pub async fn cleanup_expired(db: &PgPool) -> Result<CleanupResult, sqlx::Error> {
    let mut tx = db.begin().await?;
    let requests = sqlx::query("DELETE FROM identity_oauth_requests WHERE handle_hash IN (SELECT handle_hash FROM identity_oauth_requests WHERE expires_at<=clock_timestamp() ORDER BY expires_at LIMIT 64 FOR UPDATE SKIP LOCKED)")
        .execute(&mut *tx).await?.rows_affected();
    let unexchanged_codes = sqlx::query("DELETE FROM identity_oauth_codes WHERE code_hash IN (SELECT c.code_hash FROM identity_oauth_codes c WHERE c.expires_at<=clock_timestamp() AND NOT EXISTS(SELECT 1 FROM identity_oauth_grants g WHERE g.code_hash=c.code_hash) ORDER BY c.expires_at LIMIT 64 FOR UPDATE OF c SKIP LOCKED)")
        .execute(&mut *tx).await?.rows_affected();
    let consents = sqlx::query("DELETE FROM identity_oauth_consents WHERE (principal_id,client_id,policy_hash) IN (SELECT principal_id,client_id,policy_hash FROM identity_oauth_consents WHERE expires_at<=clock_timestamp() ORDER BY expires_at LIMIT 64 FOR UPDATE SKIP LOCKED)")
        .execute(&mut *tx).await?.rows_affected();
    tx.commit().await?;
    Ok(CleanupResult {
        requests,
        unexchanged_codes,
        consents,
    })
}
