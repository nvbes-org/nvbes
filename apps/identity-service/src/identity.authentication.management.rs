use sqlx::{Postgres, Transaction};
use uuid::Uuid;

/// Serializes factor management on the principal before locking the session.
pub(crate) async fn owner(
    tx: &mut Transaction<'_, Postgres>,
    token: &str,
    strong: bool,
) -> Result<Option<Uuid>, sqlx::Error> {
    crate::session_locks::session(tx, token).await?;
    sqlx::query_scalar("SELECT s.principal_id FROM identity_sessions s JOIN identity_principals p ON p.id=s.principal_id WHERE s.token_hash=$1 AND s.revoked_at IS NULL AND s.expires_at>clock_timestamp() AND p.status='active' AND (NOT $2 OR (s.primary_amr='webauthn' AND s.authenticated_at>clock_timestamp()-interval '5 minutes') OR (s.step_up_method IN ('totp','webauthn') AND s.step_up_at>clock_timestamp()-interval '5 minutes' AND s.step_up_expires_at>clock_timestamp())) FOR UPDATE OF s,p")
        .bind(crate::auth::hash_token(token)).bind(strong).fetch_optional(&mut **tx).await
}
