use super::{WebauthnError, valid_credential_label};
use crate::oauth::store;
use sqlx::PgPool;
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
    let (session, principal) = crate::enrollment_policy::owner(&mut tx, session_token)
        .await?
        .ok_or(WebauthnError::InvalidSession)?;
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
    let (session, principal) = crate::enrollment_policy::owner(&mut tx, session_token)
        .await?
        .ok_or(WebauthnError::InvalidSession)?;
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
    // The challenge lock and verification may outlive enrollment authorization.
    if crate::enrollment_policy::owner(&mut tx, session_token).await? != Some((session, principal))
    {
        return Err(WebauthnError::InvalidSession);
    }
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO identity_webauthn_credentials(id,principal_id,credential_id,public_key,passkey,label,discoverable) VALUES($1,$2,$3,$4,$5,$6,NULL)")
        .bind(id).bind(principal).bind(passkey.cred_id().as_ref())
        .bind(serde_json::to_value(passkey.get_public_key())?)
        .bind(serde_json::to_value(&passkey)?).bind(label).execute(&mut *tx).await?;
    store::audit(&mut tx, principal, "identity.webauthn.enrolled").await?;
    if !super::challenge::consume(&mut tx, ceremony_id, principal, "registration").await? {
        return Err(WebauthnError::InvalidCeremony);
    }
    tx.commit().await?;
    Ok(id)
}

#[cfg(all(test, feature = "database-tests"))]
#[path = "identity.webauthn.registration.tests.rs"]
mod tests;
