use sqlx::{Postgres, Transaction};
use uuid::Uuid;

/// Fresh primary authentication enrolls the first factor. Once a strong factor
/// exists, adding another requires recent proof of a strong factor.
pub(crate) async fn owner(
    tx: &mut Transaction<'_, Postgres>,
    token: &str,
) -> Result<Option<(Uuid, Uuid)>, sqlx::Error> {
    let Some(evidence) = crate::authentication_session::load(tx, token).await? else {
        return Ok(None);
    };
    if !evidence.recent_primary && !evidence.recent_strong {
        return Ok(None);
    }
    let principal = evidence.principal;
    let has_factor: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM identity_webauthn_credentials WHERE principal_id=$1 AND revoked_at IS NULL) OR EXISTS(SELECT 1 FROM identity_auth_factors WHERE principal_id=$1 AND state='active')")
        .bind(principal).fetch_one(&mut **tx).await?;
    Ok((!has_factor || evidence.recent_strong).then_some((evidence.session, principal)))
}
