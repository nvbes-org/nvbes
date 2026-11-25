use chrono::{DateTime, Utc};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use super::{
    error::OAuthError,
    request::AuthorizationRequest,
    store::{StoreError, hash},
};
use crate::authentication::Authentication;

pub(super) struct ActiveSession {
    pub id: Uuid,
    pub principal_id: Uuid,
    pub authentication: Authentication,
    /// Set when the session was presented from another environment.
    pub reauth_required: bool,
}

pub(super) async fn load(
    tx: &mut Transaction<'_, Postgres>,
    token: Option<&str>,
) -> Result<Option<ActiveSession>, StoreError> {
    let Some(token) = token else { return Ok(None) };
    crate::session_locks::session(tx, token).await?;
    let row: Option<(Uuid, Uuid, serde_json::Value, bool)> = sqlx::query_as(
        "SELECT s.id,s.principal_id,jsonb_build_object('authenticated_at',s.authenticated_at,'primary_amr',s.primary_amr,'step_up_method',s.step_up_method,'step_up_at',s.step_up_at,'step_up_expires_at',s.step_up_expires_at),s.reauth_required_at IS NOT NULL FROM identity_sessions s JOIN identity_principals p ON p.id=s.principal_id WHERE s.token_hash=$1 AND s.revoked_at IS NULL AND s.expires_at>clock_timestamp() AND p.status='active' FOR UPDATE OF s,p"
    ).bind(hash(token)).fetch_optional(&mut **tx).await?;
    let Some((id, principal_id, value, reauth_required)) = row else {
        return Ok(None);
    };
    let authentication: Authentication = serde_json::from_value(value)?;
    authentication
        .validate(Utc::now())
        .map_err(|_| OAuthError::AccessDenied)?;
    Ok(Some(ActiveSession {
        id,
        principal_id,
        authentication,
        reauth_required,
    }))
}

pub(super) fn is_fresh(
    session: &ActiveSession,
    request: &AuthorizationRequest,
    started_at: DateTime<Utc>,
) -> bool {
    if session.reauth_required {
        return false;
    }
    let at = session.authentication.authenticated_at;
    if (request.prompt.as_deref() == Some("login") || request.max_age == Some(0)) && at < started_at
    {
        return false;
    }
    request
        .max_age
        .is_none_or(|max| max == 0 || (Utc::now() - at).num_seconds() <= i64::from(max))
}
