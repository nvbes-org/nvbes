use crate::{
    authentication::Authentication,
    oauth::{clients::ClientRegistry, request::AuthorizationRequest},
    tokens_error::TokenError,
};
use chrono::{DateTime, Utc};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

pub(crate) struct ActiveGrant {
    pub id: Uuid,
    pub session_id: Uuid,
    pub principal_id: Uuid,
    pub request: AuthorizationRequest,
    pub authentication: Authentication,
    pub session_expires_at: DateTime<Utc>,
    pub token_issued_at: Option<DateTime<Utc>>,
}

#[derive(sqlx::FromRow)]
struct StoredGrant {
    session_id: Uuid,
    principal_id: Uuid,
    parameters: serde_json::Value,
    authentication: Option<serde_json::Value>,
    session_expires_at: DateTime<Utc>,
    token_issued_at: Option<DateTime<Utc>>,
}

/// Locks the grant and its active session/principal through signing and commit.
pub(crate) async fn load(
    tx: &mut Transaction<'_, Postgres>,
    clients: &ClientRegistry,
    id: Uuid,
) -> Result<ActiveGrant, TokenError> {
    let owner: Option<Uuid> =
        sqlx::query_scalar("SELECT principal_id FROM identity_oauth_grants WHERE id=$1")
            .bind(id)
            .fetch_optional(&mut **tx)
            .await?;
    crate::session_locks::principal(tx, owner.ok_or(TokenError::InactiveGrant)?).await?;
    let stored: StoredGrant = sqlx::query_as("SELECT g.session_id,g.principal_id,g.parameters,c.authentication,s.expires_at AS session_expires_at,g.token_issued_at FROM identity_oauth_grants g JOIN identity_oauth_codes c ON c.code_hash=g.code_hash JOIN identity_sessions s ON s.id=g.session_id AND s.principal_id=g.principal_id JOIN identity_principals p ON p.id=g.principal_id WHERE g.id=$1 AND g.revoked_at IS NULL AND s.revoked_at IS NULL AND s.expires_at>clock_timestamp() AND p.status='active' FOR UPDATE OF g,s,p")
        .bind(id).fetch_optional(&mut **tx).await?.ok_or(TokenError::InactiveGrant)?;
    let request = AuthorizationRequest::restore(stored.parameters, clients)?;
    let authentication: Authentication = serde_json::from_value(
        stored
            .authentication
            .ok_or(TokenError::InvalidAuthentication)?,
    )
    .map_err(|_| TokenError::InvalidAuthentication)?;
    authentication.validate(Utc::now())?;
    Ok(ActiveGrant {
        id,
        session_id: stored.session_id,
        principal_id: stored.principal_id,
        request,
        authentication,
        session_expires_at: stored.session_expires_at,
        token_issued_at: stored.token_issued_at,
    })
}
