use super::{WebauthnError, next_sign_count};
use crate::oauth::store;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;
use webauthn_rs::{Webauthn, prelude::*};

#[derive(serde::Serialize)]
pub struct StepUpOptions {
    pub ceremony_id: Uuid,
    pub options: RequestChallengeResponse,
}

/// The HTTP adapter must enforce session CSRF, Origin and account/source quotas.
pub async fn start(
    db: &PgPool,
    server: &Webauthn,
    token: &str,
) -> Result<StepUpOptions, WebauthnError> {
    let mut tx = db.begin().await?;
    let (session, principal) = active_session(&mut tx, token).await?;
    let values: Vec<serde_json::Value> = sqlx::query_scalar("SELECT passkey FROM identity_webauthn_credentials WHERE principal_id=$1 AND revoked_at IS NULL AND passkey IS NOT NULL ORDER BY id LIMIT 11")
        .bind(principal).fetch_all(&mut *tx).await?;
    if values.is_empty() || values.len() > 10 {
        return Err(WebauthnError::InvalidCeremony);
    }
    let passkeys = values
        .into_iter()
        .map(serde_json::from_value)
        .collect::<Result<Vec<Passkey>, _>>()?;
    let (options, state) = server
        .start_passkey_authentication(&passkeys)
        .map_err(|_| WebauthnError::InvalidCeremony)?;
    let id = Uuid::new_v4();
    sqlx::query(
        "DELETE FROM identity_webauthn_challenges WHERE session_id=$1 AND purpose='step_up'",
    )
    .bind(session)
    .execute(&mut *tx)
    .await?;
    sqlx::query("INSERT INTO identity_webauthn_challenges(id,principal_id,session_id,challenge,purpose,expires_at,ceremony_state) VALUES($1,$2,$3,$4,'step_up',clock_timestamp()+interval '5 minutes',$5)")
        .bind(id).bind(principal).bind(session).bind(options.public_key.challenge.as_ref())
        .bind(serde_json::to_value(state)?).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(StepUpOptions {
        ceremony_id: id,
        options,
    })
}

pub async fn finish(
    db: &PgPool,
    server: &Webauthn,
    token: &str,
    ceremony: Uuid,
    response: &PublicKeyCredential,
) -> Result<DateTime<Utc>, WebauthnError> {
    let mut tx = db.begin().await?;
    let (session, principal) = active_session(&mut tx, token).await?;
    let state: Option<serde_json::Value> = sqlx::query_scalar("SELECT ceremony_state FROM identity_webauthn_challenges WHERE id=$1 AND session_id=$2 AND principal_id=$3 AND purpose='step_up' AND consumed_at IS NULL AND expires_at>clock_timestamp() AND ceremony_state IS NOT NULL FOR UPDATE")
        .bind(ceremony).bind(session).bind(principal).fetch_optional(&mut *tx).await?;
    let state: PasskeyAuthentication =
        serde_json::from_value(state.ok_or(WebauthnError::InvalidCeremony)?)?;
    let verified = server
        .finish_passkey_authentication(response, &state)
        .map_err(|_| WebauthnError::InvalidCeremony)?;
    if !verified.user_verified() {
        return Err(WebauthnError::InvalidCeremony);
    }
    // A credential can be revoked or used since the ceremony began. Validate
    // against the current locked record, not only the snapshot in the challenge.
    let stored: Option<(Uuid, serde_json::Value)> = sqlx::query_as("SELECT id,passkey FROM identity_webauthn_credentials WHERE principal_id=$1 AND credential_id=$2 AND revoked_at IS NULL AND passkey IS NOT NULL FOR UPDATE")
        .bind(principal).bind(verified.cred_id().as_ref()).fetch_optional(&mut *tx).await?;
    let (id, value) = stored.ok_or(WebauthnError::InvalidCeremony)?;
    let mut passkey: Passkey = serde_json::from_value(value)?;
    let current: Credential = passkey.clone().into();
    next_sign_count(current.counter, verified.counter())
        .map_err(|_| WebauthnError::InvalidCeremony)?;
    if passkey.update_credential(&verified).is_none() {
        return Err(WebauthnError::InvalidCeremony);
    }
    sqlx::query("UPDATE identity_webauthn_credentials SET passkey=$1,sign_count=$2,backed_up=$3,last_used_at=clock_timestamp() WHERE id=$4")
        .bind(serde_json::to_value(passkey)?).bind(i64::from(verified.counter())).bind(verified.backup_state()).bind(id).execute(&mut *tx).await?;
    sqlx::query(
        "UPDATE identity_webauthn_challenges SET consumed_at=clock_timestamp() WHERE id=$1",
    )
    .bind(ceremony)
    .execute(&mut *tx)
    .await?;
    let expires: DateTime<Utc> = sqlx::query_scalar("UPDATE identity_sessions SET step_up_method='webauthn',step_up_at=clock_timestamp(),step_up_expires_at=LEAST(expires_at,clock_timestamp()+interval '10 minutes') WHERE id=$1 RETURNING step_up_expires_at")
        .bind(session).fetch_one(&mut *tx).await?;
    sqlx::query("INSERT INTO identity_session_webauthn_credentials(session_id,credential_id,principal_id,purpose) VALUES($1,$2,$3,'step_up') ON CONFLICT (session_id,credential_id,purpose) DO UPDATE SET authenticated_at=EXCLUDED.authenticated_at")
        .bind(session).bind(id).bind(principal).execute(&mut *tx).await?;
    store::audit(&mut tx, principal, "identity.step_up_granted").await?;
    tx.commit().await?;
    Ok(expires)
}

async fn active_session(
    tx: &mut Transaction<'_, Postgres>,
    token: &str,
) -> Result<(Uuid, Uuid), WebauthnError> {
    sqlx::query_as("SELECT s.id,s.principal_id FROM identity_sessions s JOIN identity_principals p ON p.id=s.principal_id WHERE s.token_hash=$1 AND s.revoked_at IS NULL AND s.expires_at>clock_timestamp() AND p.status='active' FOR UPDATE OF s,p")
        .bind(store::hash(token)).fetch_optional(&mut **tx).await?.ok_or(WebauthnError::InvalidSession)
}

#[cfg(all(test, feature = "database-tests"))]
#[path = "identity.webauthn.step_up.tests.rs"]
mod tests;
