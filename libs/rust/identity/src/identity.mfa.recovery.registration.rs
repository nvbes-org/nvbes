use super::{RecoveryError, event, owner};
use crate::webauthn::{registration::RegistrationOptions, valid_credential_label};
use sqlx::PgPool;
use uuid::Uuid;
use webauthn_rs::{Webauthn, prelude::*};

/// The recovery authorization can only create a new passkey, never an OAuth session.
pub async fn start(
    db: &PgPool,
    server: &Webauthn,
    token: &str,
) -> Result<RegistrationOptions, RecoveryError> {
    let mut tx = db.begin().await?;
    let (session, principal) = owner(&mut tx, token).await?;
    let existing: Vec<Vec<u8>>=sqlx::query_scalar("SELECT credential_id FROM identity_webauthn_credentials WHERE principal_id=$1 AND revoked_at IS NULL ORDER BY id LIMIT 10")
        .bind(principal).fetch_all(&mut *tx).await?;
    let (mut options, state) = server
        .start_passkey_registration(
            principal,
            &principal.to_string(),
            "nvbes account",
            Some(existing.into_iter().map(Into::into).collect()),
        )
        .map_err(|_| RecoveryError::Invalid)?;
    options
        .public_key
        .authenticator_selection
        .as_mut()
        .ok_or(RecoveryError::Invalid)?
        .resident_key = Some(webauthn_rs_proto::ResidentKeyRequirement::Preferred);
    sqlx::query("DELETE FROM identity_webauthn_challenges WHERE recovery_session_id=$1")
        .bind(session)
        .execute(&mut *tx)
        .await?;
    let ceremony = Uuid::new_v4();
    sqlx::query("INSERT INTO identity_webauthn_challenges(id,principal_id,challenge,purpose,expires_at,ceremony_state,recovery_session_id) SELECT $1,principal_id,$2,'registration',expires_at,$3,id FROM identity_mfa_recovery_sessions WHERE id=$4")
        .bind(ceremony).bind(options.public_key.challenge.as_ref()).bind(serde_json::to_value(state)?).bind(session).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(RegistrationOptions {
        ceremony_id: ceremony,
        options,
    })
}

/// Proof of the new key is verified before replacing any old authentication factor.
pub async fn finish(
    db: &PgPool,
    server: &Webauthn,
    token: &str,
    ceremony: Uuid,
    response: &RegisterPublicKeyCredential,
    label: &str,
) -> Result<Uuid, RecoveryError> {
    if !valid_credential_label(label) {
        return Err(RecoveryError::Invalid);
    }
    let mut tx = db.begin().await?;
    let (session, principal) = owner(&mut tx, token).await?;
    let state: Option<serde_json::Value>=sqlx::query_scalar("SELECT ceremony_state FROM identity_webauthn_challenges WHERE id=$1 AND recovery_session_id=$2 AND principal_id=$3 AND purpose='registration' AND consumed_at IS NULL AND expires_at>clock_timestamp() FOR UPDATE")
        .bind(ceremony).bind(session).bind(principal).fetch_optional(&mut *tx).await?;
    let state: PasskeyRegistration = serde_json::from_value(state.ok_or(RecoveryError::Invalid)?)?;
    let passkey = server
        .finish_passkey_registration(response, &state)
        .map_err(|_| RecoveryError::Invalid)?;
    let id = Uuid::new_v4();
    sqlx::query("UPDATE identity_webauthn_credentials SET revoked_at=clock_timestamp() WHERE principal_id=$1 AND revoked_at IS NULL").bind(principal).execute(&mut *tx).await?;
    sqlx::query("UPDATE identity_auth_factors SET state='revoked',revoked_at=clock_timestamp(),secret_ciphertext=''::bytea,enrollment_session_id=NULL,enrollment_expires_at=NULL WHERE principal_id=$1 AND state<>'revoked'").bind(principal).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO identity_webauthn_credentials(id,principal_id,credential_id,public_key,passkey,label,discoverable) VALUES($1,$2,$3,$4,$5,$6,NULL)")
        .bind(id).bind(principal).bind(passkey.cred_id().as_ref()).bind(serde_json::to_value(passkey.get_public_key())?).bind(serde_json::to_value(&passkey)?).bind(label).execute(&mut *tx).await?;
    // Revoke also sessions created by another login while recovery was in progress.
    sqlx::query("UPDATE identity_sessions SET revoked_at=clock_timestamp() WHERE principal_id=$1 AND revoked_at IS NULL").bind(principal).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM identity_mfa_recovery_codes WHERE principal_id=$1")
        .bind(principal)
        .execute(&mut *tx)
        .await?;
    event(&mut tx, principal, "identity.mfa_recovered").await?;
    // Revocation and notification writes can wait too; expiry invalidates the
    // entire replacement, including those writes, until this authorization point.
    if owner(&mut tx, token).await? != (session, principal)
        || !crate::webauthn::challenge::consume(&mut tx, ceremony, principal, "registration")
            .await?
    {
        return Err(RecoveryError::Invalid);
    }
    sqlx::query("DELETE FROM identity_mfa_recovery_sessions WHERE id=$1")
        .bind(session)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(id)
}
