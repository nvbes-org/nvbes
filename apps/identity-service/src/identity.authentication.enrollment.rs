use sqlx::{Postgres, Transaction};
use uuid::Uuid;

/// Fresh primary authentication enrolls the first factor. Once a strong factor
/// exists, adding another requires recent proof of a strong factor.
pub(crate) async fn owner(
    tx: &mut Transaction<'_, Postgres>,
    token: &str,
) -> Result<Option<(Uuid, Uuid)>, sqlx::Error> {
    crate::session_locks::session(tx, token).await?;
    let row: Option<(Uuid, Uuid, bool)> = sqlx::query_as("SELECT s.id,s.principal_id,COALESCE((s.primary_amr='webauthn' AND s.authenticated_at>clock_timestamp()-interval '5 minutes') OR (s.step_up_method IN ('totp','webauthn') AND s.step_up_at>clock_timestamp()-interval '5 minutes' AND s.step_up_expires_at>clock_timestamp()),false) FROM identity_sessions s JOIN identity_principals p ON p.id=s.principal_id WHERE s.token_hash=$1 AND s.revoked_at IS NULL AND s.expires_at>clock_timestamp() AND p.status='active' AND (s.authenticated_at>clock_timestamp()-interval '5 minutes' OR (s.step_up_at>clock_timestamp()-interval '5 minutes' AND s.step_up_expires_at>clock_timestamp())) FOR UPDATE OF s,p")
        .bind(crate::auth::hash_token(token)).fetch_optional(&mut **tx).await?;
    let Some((session, principal, strong)) = row else {
        return Ok(None);
    };
    let has_factor: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM identity_webauthn_credentials WHERE principal_id=$1 AND revoked_at IS NULL) OR EXISTS(SELECT 1 FROM identity_auth_factors WHERE principal_id=$1 AND state='active')")
        .bind(principal).fetch_one(&mut **tx).await?;
    Ok((!has_factor || strong).then_some((session, principal)))
}
