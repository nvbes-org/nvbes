use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;

use super::{
    authentication_policy::AuthenticationPolicy, clients::ClientRegistry, interactions,
    request::AuthorizationRequest, session, store::StoreError,
};
use crate::browser::BrowserProof;

#[derive(Serialize)]
pub struct AuthenticationStatus {
    pub minimum_authentication: AuthenticationPolicy,
    pub needs_login: bool,
    pub needs_step_up: bool,
    pub proof_expires_at: Option<DateTime<Utc>>,
}

/// Read current policy and bound-session evidence without consuming the interaction.
/// This is display information, never an authorization proof for consent.
pub async fn read(
    db: &PgPool,
    clients: &ClientRegistry,
    handle: &str,
    proof: &BrowserProof,
) -> Result<AuthenticationStatus, StoreError> {
    let mut tx = db.begin().await?;
    let stored = interactions::lock(
        &mut tx,
        handle,
        &proof.browser_token,
        Some(&proof.csrf_token),
    )
    .await?;
    let request = AuthorizationRequest::restore(stored.parameters, clients)?;
    let session = session::load(&mut tx, proof.session_token.as_deref())
        .await?
        .filter(|session| {
            Some(session.id) == stored.bound_session_id
                && session::is_fresh(session, &request, stored.created_at)
        });
    let result = session.as_ref().map(|session| {
        request
            .minimum_authentication
            .deadline(&session.authentication, Utc::now())
    });
    let status = AuthenticationStatus {
        minimum_authentication: request.minimum_authentication,
        needs_login: session.is_none(),
        needs_step_up: result.as_ref().is_some_and(Result::is_err),
        proof_expires_at: result.and_then(Result::ok).flatten(),
    };
    tx.commit().await?;
    Ok(status)
}
