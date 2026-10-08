use chrono::{DateTime, Duration, Utc};
use sqlx::{Postgres, Transaction, types::Json};
use uuid::Uuid;

use crate::authentication::Authentication;

pub(crate) struct SessionEvidence {
    pub session: Uuid,
    pub principal: Uuid,
    pub recent_primary: bool,
    pub recent_strong: bool,
}

/// Validate the same evidence as token issuance, against the database clock.
/// The principal lock precedes session locking and all authorization decisions.
pub(crate) async fn load(
    tx: &mut Transaction<'_, Postgres>,
    token: &str,
) -> Result<Option<SessionEvidence>, sqlx::Error> {
    crate::session_locks::session(tx, token).await?;
    let row: Option<(Uuid, Uuid, Json<Authentication>, DateTime<Utc>)> = sqlx::query_as(
        "SELECT s.id,s.principal_id,jsonb_build_object('authenticated_at',s.authenticated_at,'primary_amr',s.primary_amr,'step_up_method',s.step_up_method,'step_up_at',s.step_up_at,'step_up_expires_at',s.step_up_expires_at),clock_timestamp() FROM identity_sessions s JOIN identity_principals p ON p.id=s.principal_id WHERE s.token_hash=$1 AND s.revoked_at IS NULL AND s.expires_at>clock_timestamp() AND p.status='active' FOR UPDATE OF s,p"
    ).bind(crate::auth::hash_token(token)).fetch_optional(&mut **tx).await?;
    let Some((session, principal, Json(authentication), now)) = row else {
        return Ok(None);
    };
    if authentication.validate(now).is_err() {
        return Ok(None);
    }
    let cutoff = now - Duration::minutes(5);
    let recent_primary = authentication.authenticated_at > cutoff;
    let recent_step_up = authentication.step_up_at.is_some_and(|at| at > cutoff)
        && authentication
            .step_up_expires_at
            .is_some_and(|until| until > now);
    Ok(Some(SessionEvidence {
        session,
        principal,
        recent_primary,
        recent_strong: (recent_primary && authentication.primary_amr == "webauthn")
            || recent_step_up,
    }))
}

#[cfg(all(test, feature = "database-tests"))]
#[path = "identity.authentication.session.tests.rs"]
mod tests;
