use super::{WebauthnError, valid_credential_label};
use crate::oauth::store;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;
use webauthn_rs::{Webauthn, prelude::*};

#[derive(serde::Serialize)]
pub struct RegistrationOptions {
    pub ceremony_id: Uuid,
    pub options: CreationChallengeResponse,
}

/// The HTTP adapter must verify Origin and session-bound CSRF before calling.
pub async fn start(
    db: &PgPool,
    server: &Webauthn,
    session_token: &str,
) -> Result<RegistrationOptions, WebauthnError> {
    let mut tx = db.begin().await?;
    let (session, principal) = enrollment_session(&mut tx, session_token).await?;
    let credentials: Vec<Vec<u8>> = sqlx::query_scalar(
        "SELECT credential_id FROM identity_webauthn_credentials WHERE principal_id=$1 AND revoked_at IS NULL",
    )
    .bind(principal)
    .fetch_all(&mut *tx)
    .await?;
    if credentials.len() >= 10 {
        return Err(WebauthnError::Limit);
    }
    let (mut options, state) = server
        .start_passkey_registration(
            principal,
            &principal.to_string(),
            "nvbes account",
            Some(credentials.into_iter().map(Into::into).collect()),
        )
        .map_err(|_| WebauthnError::InvalidCeremony)?;
    // Prefer discoverability without requiring it: classic security keys remain
    // accepted, exactly as in the stored library policy. UV is never weakened.
    let selection = options
        .public_key
        .authenticator_selection
        .as_mut()
        .ok_or(WebauthnError::InvalidCeremony)?;
    selection.resident_key = Some(webauthn_rs_proto::ResidentKeyRequirement::Preferred);
    let id = Uuid::new_v4();
    // One pending enrollment per session. Session locking serializes starts.
    sqlx::query(
        "DELETE FROM identity_webauthn_challenges WHERE session_id=$1 AND purpose='registration'",
    )
    .bind(session)
    .execute(&mut *tx)
    .await?;
    sqlx::query("INSERT INTO identity_webauthn_challenges(id,principal_id,session_id,challenge,purpose,expires_at,ceremony_state) VALUES($1,$2,$3,$4,'registration',clock_timestamp()+interval '5 minutes',$5)")
        .bind(id).bind(principal).bind(session)
        .bind(options.public_key.challenge.as_ref())
        .bind(serde_json::to_value(state)?).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(RegistrationOptions {
        ceremony_id: id,
        options,
    })
}

pub async fn finish(
    db: &PgPool,
    server: &Webauthn,
    session_token: &str,
    ceremony_id: Uuid,
    response: &RegisterPublicKeyCredential,
    label: &str,
) -> Result<Uuid, WebauthnError> {
    if !valid_credential_label(label) {
        return Err(WebauthnError::InvalidCeremony);
    }
    let mut tx = db.begin().await?;
    let (session, principal) = enrollment_session(&mut tx, session_token).await?;
    let state: Option<serde_json::Value> = sqlx::query_scalar("SELECT ceremony_state FROM identity_webauthn_challenges WHERE id=$1 AND session_id=$2 AND principal_id=$3 AND purpose='registration' AND consumed_at IS NULL AND expires_at>clock_timestamp() AND ceremony_state IS NOT NULL FOR UPDATE")
        .bind(ceremony_id).bind(session).bind(principal).fetch_optional(&mut *tx).await?;
    let state: PasskeyRegistration =
        serde_json::from_value(state.ok_or(WebauthnError::InvalidCeremony)?)?;
    let passkey = server
        .finish_passkey_registration(response, &state)
        .map_err(|_| WebauthnError::InvalidCeremony)?;
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM identity_webauthn_credentials WHERE principal_id=$1 AND revoked_at IS NULL",
    )
    .bind(principal)
    .fetch_one(&mut *tx)
    .await?;
    if count >= 10 {
        return Err(WebauthnError::Limit);
    }
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO identity_webauthn_credentials(id,principal_id,credential_id,public_key,passkey,label,discoverable) VALUES($1,$2,$3,$4,$5,$6,NULL)")
        .bind(id).bind(principal).bind(passkey.cred_id().as_ref())
        .bind(serde_json::to_value(passkey.get_public_key())?)
        .bind(serde_json::to_value(&passkey)?).bind(label).execute(&mut *tx).await?;
    sqlx::query(
        "UPDATE identity_webauthn_challenges SET consumed_at=clock_timestamp() WHERE id=$1",
    )
    .bind(ceremony_id)
    .execute(&mut *tx)
    .await?;
    store::audit(&mut tx, principal, "identity.webauthn.enrolled").await?;
    tx.commit().await?;
    Ok(id)
}

async fn enrollment_session(
    tx: &mut Transaction<'_, Postgres>,
    token: &str,
) -> Result<(Uuid, Uuid), WebauthnError> {
    // Lock the principal as well: the per-account credential cap holds across
    // simultaneous enrollments from different sessions.
    let row: Option<(Uuid, Uuid, bool)> = sqlx::query_as("SELECT s.id,s.principal_id,COALESCE((s.primary_amr='webauthn' AND s.authenticated_at>clock_timestamp()-interval '5 minutes') OR (s.step_up_method IN ('totp','webauthn') AND s.step_up_at>clock_timestamp()-interval '5 minutes' AND s.step_up_expires_at>clock_timestamp()),false) FROM identity_sessions s JOIN identity_principals p ON p.id=s.principal_id WHERE s.token_hash=$1 AND s.revoked_at IS NULL AND s.expires_at>clock_timestamp() AND p.status='active' AND (s.authenticated_at>clock_timestamp()-interval '5 minutes' OR (s.step_up_at>clock_timestamp()-interval '5 minutes' AND s.step_up_expires_at>clock_timestamp())) FOR UPDATE OF s,p")
        .bind(store::hash(token)).fetch_optional(&mut **tx).await?;
    let (session, principal, strong) = row.ok_or(WebauthnError::InvalidSession)?;
    let has_factor: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM identity_webauthn_credentials WHERE principal_id=$1 AND revoked_at IS NULL) OR EXISTS(SELECT 1 FROM identity_auth_factors WHERE principal_id=$1 AND state='active')")
        .bind(principal).fetch_one(&mut **tx).await?;
    if has_factor && !strong {
        return Err(WebauthnError::InvalidSession);
    }
    Ok((session, principal))
}

#[cfg(all(test, feature = "database-tests"))]
#[path = "identity.webauthn.registration.tests.rs"]
mod tests;
