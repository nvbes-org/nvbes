use sqlx::{Postgres, Transaction};
use uuid::Uuid;

/// Acquire the principal before any session or authentication-factor row lock.
/// This serializes multi-session revocation with authentication and token issuance.
pub(crate) async fn principal(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT id FROM identity_principals WHERE id=$1 FOR UPDATE")
        .bind(id)
        .fetch_optional(&mut **tx)
        .await?;
    Ok(())
}

/// The lookup is only a locking hint; callers must recheck session ownership,
/// activity and authorization after this lock has been acquired.
pub(crate) async fn session(
    tx: &mut Transaction<'_, Postgres>,
    token: &str,
) -> Result<(), sqlx::Error> {
    let owner: Option<Uuid> =
        sqlx::query_scalar("SELECT principal_id FROM identity_sessions WHERE token_hash=$1")
            .bind(crate::auth::hash_token(token))
            .fetch_optional(&mut **tx)
            .await?;
    if let Some(owner) = owner {
        principal(tx, owner).await?;
    }
    Ok(())
}
