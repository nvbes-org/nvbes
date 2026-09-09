use chrono::{DateTime, Utc};
use rand::RngCore;
use reqwest::Url;
use sqlx::PgPool;
use uuid::Uuid;
use webauthn_rs::{
    Webauthn, WebauthnBuilder,
    prelude::{CreationChallengeResponse, PasskeyRegistration},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChallengePurpose {
    Registration,
    Authentication,
    StepUp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthenticatorAssurance {
    UserVerified,
    UserVerifiedWithBackupSignals,
}

/// Backup eligibility or discoverability are recorded as signals only. They do
/// not, on their own, prove a higher assurance level.
pub fn assurance_from_signals(
    user_verified: bool,
    backed_up: bool,
) -> Option<AuthenticatorAssurance> {
    if !user_verified {
        return None;
    }
    Some(if backed_up {
        AuthenticatorAssurance::UserVerifiedWithBackupSignals
    } else {
        AuthenticatorAssurance::UserVerified
    })
}

pub fn build_server(rp_id: &str, rp_origin: &str) -> Result<Webauthn, String> {
    let origin = Url::parse(rp_origin).map_err(|_| "invalid WebAuthn RP origin".to_owned())?;
    WebauthnBuilder::new(rp_id, &origin)
        .map_err(|_| "invalid WebAuthn RP configuration".to_owned())?
        .build()
        .map_err(|_| "invalid WebAuthn RP configuration".to_owned())
}

pub fn start_passkey_registration(
    server: &Webauthn,
    principal_id: Uuid,
    user_name: &str,
    display_name: &str,
) -> Result<(CreationChallengeResponse, PasskeyRegistration), String> {
    if user_name.is_empty() || display_name.is_empty() {
        return Err("WebAuthn user identity is required".to_owned());
    }
    server
        .start_passkey_registration(principal_id, user_name, display_name, None)
        .map_err(|_| "WebAuthn registration could not start".to_owned())
}

impl ChallengePurpose {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Registration => "registration",
            Self::Authentication => "authentication",
            Self::StepUp => "step_up",
        }
    }
}

pub fn challenge_is_live(expires_at: DateTime<Utc>, now: DateTime<Utc>) -> bool {
    expires_at > now
}

pub fn generate_challenge() -> [u8; 32] {
    let mut challenge = [0_u8; 32];
    rand::rng().fill_bytes(&mut challenge);
    challenge
}

pub fn valid_credential_label(label: &str) -> bool {
    (1..=128).contains(&label.chars().count()) && label.chars().all(|value| !value.is_control())
}

pub fn valid_credential_id(credential_id: &[u8]) -> bool {
    (16..=1024).contains(&credential_id.len())
}

pub async fn store_challenge(
    db: &PgPool,
    principal_id: Option<Uuid>,
    purpose: ChallengePurpose,
    expires_at: DateTime<Utc>,
) -> Result<([u8; 32], Uuid), sqlx::Error> {
    let challenge = generate_challenge();
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO identity_webauthn_challenges(id,principal_id,challenge,purpose,expires_at) VALUES($1,$2,$3,$4,$5)")
        .bind(id).bind(principal_id).bind(challenge.as_slice())
        .bind(purpose.as_str()).bind(expires_at).execute(db).await?;
    Ok((challenge, id))
}

pub async fn consume_challenge(
    db: &PgPool,
    challenge: &[u8; 32],
    expected_purpose: ChallengePurpose,
    principal_id: Option<Uuid>,
) -> Result<Uuid, sqlx::Error> {
    let id: Option<Uuid> = sqlx::query_scalar("UPDATE identity_webauthn_challenges SET consumed_at=clock_timestamp() WHERE challenge=$1 AND purpose=$2 AND consumed_at IS NULL AND expires_at>clock_timestamp() AND (principal_id IS NOT DISTINCT FROM $3) RETURNING id")
        .bind(challenge.as_slice()).bind(expected_purpose.as_str()).bind(principal_id)
        .fetch_optional(db).await?;
    id.ok_or(sqlx::Error::RowNotFound)
}

/// WebAuthn authenticators may report zero permanently. Once a non-zero
/// counter has been stored, a decrease is a rollback signal and is rejected.
pub fn next_sign_count(previous: u32, reported: u32) -> Result<u32, ()> {
    if previous > 0 && reported <= previous {
        return Err(());
    }
    Ok(reported)
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};

    use super::{challenge_is_live, next_sign_count};

    #[test]
    fn challenge_expiry_is_strict() {
        let now = Utc::now();
        assert!(challenge_is_live(now + Duration::seconds(1), now));
        assert!(!challenge_is_live(now, now));
    }

    #[test]
    fn signature_counter_allows_zero_but_rejects_rollback() {
        assert_eq!(next_sign_count(0, 0), Ok(0));
        assert_eq!(next_sign_count(0, 4), Ok(4));
        assert_eq!(next_sign_count(4, 5), Ok(5));
        assert_eq!(next_sign_count(4, 4), Err(()));
        assert_eq!(next_sign_count(4, 3), Err(()));
    }

    #[test]
    fn credential_labels_are_bounded_and_printable() {
        assert!(super::valid_credential_label("MacBook passkey"));
        assert!(!super::valid_credential_label(""));
        assert!(!super::valid_credential_label("bad\nlabel"));
        assert!(!super::valid_credential_label(&"x".repeat(129)));
    }

    #[test]
    fn credential_ids_are_bounded() {
        assert!(!super::valid_credential_id(&[0; 15]));
        assert!(super::valid_credential_id(&[0; 16]));
        assert!(super::valid_credential_id(&[0; 1024]));
        assert!(!super::valid_credential_id(&[0; 1025]));
    }

    #[test]
    fn backup_signals_do_not_become_an_aal_claim() {
        assert_eq!(super::assurance_from_signals(false, true), None);
        assert_eq!(
            super::assurance_from_signals(true, true),
            Some(super::AuthenticatorAssurance::UserVerifiedWithBackupSignals)
        );
    }
}
