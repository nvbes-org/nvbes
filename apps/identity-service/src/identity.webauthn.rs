use chrono::{DateTime, Utc};
use rand::RngCore;
use reqwest::Url;
use webauthn_rs::{
    Webauthn, WebauthnBuilder,
    prelude::{Passkey, PasskeyAuthentication, RequestChallengeResponse},
};

#[path = "identity.webauthn.registration.rs"]
pub mod registration;

#[path = "identity.webauthn.credentials.rs"]
pub mod credentials;
#[path = "identity.webauthn.step_up.rs"]
pub mod step_up;

#[derive(Debug, thiserror::Error)]
pub enum WebauthnError {
    #[error("invalid or expired WebAuthn ceremony")]
    InvalidCeremony,
    #[error("fresh active session required")]
    InvalidSession,
    #[error("credential limit reached")]
    Limit,
    #[error("another active strong factor is required")]
    LastFactor,
    #[error("WebAuthn persistence unavailable")]
    Database(#[from] sqlx::Error),
    #[error("invalid stored WebAuthn state")]
    Serialization(#[from] serde_json::Error),
}

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

pub fn start_passkey_authentication(
    server: &Webauthn,
    credentials: serde_json::Value,
) -> Result<(RequestChallengeResponse, PasskeyAuthentication), String> {
    let credentials: Vec<Passkey> = serde_json::from_value(credentials)
        .map_err(|_| "invalid stored WebAuthn credentials".to_owned())?;
    if credentials.is_empty() {
        return Err("no WebAuthn credentials are registered".to_owned());
    }
    server
        .start_passkey_authentication(&credentials)
        .map_err(|_| "WebAuthn authentication could not start".to_owned())
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
