use nvbes_core::config::AppConfig;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;
use webauthn_rs::prelude::PublicKeyCredential;

use super::super::login_challenges::{self, CreateLoginChallengeInput};
use super::super::risk::{self, RiskEventInput};
use super::types::StoredPasskeyAuthentication;
use super::{
    errors::map_webauthn_authentication_error,
    storage::{load_passkeys, passkey_credentials, record_passkey_authentication},
};
use crate::http::error::AppError;

#[path = "identity.domains.auth.webauthn.login.discoverable.rs"]
mod discoverable;

pub use discoverable::{
    finish_discoverable_login_authentication, start_discoverable_login_authentication,
};

pub async fn start_login_authentication(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    config: &AppConfig,
    webauthn: &webauthn_rs::Webauthn,
    auth_state_id: Uuid,
    principal_id: Uuid,
    ip: Option<&str>,
    user_agent: Option<&str>,
) -> Result<(Uuid, serde_json::Value), AppError> {
    let passkeys = load_passkeys(db, principal_id).await?;
    if passkeys.is_empty() {
        return Err(AppError::forbidden(
            "webauthn_not_configured",
            "No active WebAuthn credentials are configured.",
        ));
    }

    let credentials = passkey_credentials(&passkeys);
    let (request, authentication) = webauthn
        .start_passkey_authentication(&credentials)
        .map_err(|_| {
            AppError::internal(
                "webauthn_auth_start_failed",
                "Failed to start WebAuthn authentication.",
            )
        })?;

    let challenge_id = login_challenges::replace_challenge(
        redis,
        CreateLoginChallengeInput {
            auth_state_id,
            principal_id: Some(principal_id),
            tenant_id: None,
            workspace_id: None,
            purpose: "webauthn_login",
            required_level: "aal2",
            allowed_factor_types: vec!["webauthn"],
            factor_id: None,
            metadata: json!({
                "authentication": StoredPasskeyAuthentication { authentication },
            }),
            ttl_minutes: 5,
        },
    )
    .await?;

    let geo_signal = risk::geo::apply_geo_security_signal(
        db,
        config,
        principal_id,
        ip,
        5.0,
        json!({
            "passkey_count": passkeys.len(),
            "auth_state_id": auth_state_id,
        }),
        "webauthn_login_started",
    )
    .await;
    let _ = risk::record_event(
        db,
        RiskEventInput {
            principal_id,
            session_id: None,
            device_id: None,
            event_type: "webauthn_login_started".to_string(),
            ip_address: ip.map(ToOwned::to_owned),
            user_agent: user_agent.map(ToOwned::to_owned),
            risk_score: geo_signal.score,
            risk_factors: geo_signal.factors,
            decision: geo_signal.decision,
            metadata: json!({ "geo": geo_signal.metadata }),
        },
    )
    .await;

    let options = serde_json::to_value(request).map_err(|_| {
        AppError::internal(
            "webauthn_serialization_failed",
            "Failed to serialize WebAuthn options.",
        )
    })?;
    let options = shape_authentication_options(options);

    Ok((challenge_id, options))
}

pub async fn finish_login_authentication(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    config: &AppConfig,
    webauthn: &webauthn_rs::Webauthn,
    auth_state_id: Uuid,
    principal_id: Uuid,
    challenge_id: Uuid,
    credential: &PublicKeyCredential,
    ip: Option<&str>,
    user_agent: Option<&str>,
) -> Result<String, AppError> {
    let challenge = login_challenges::fetch_active_challenge(
        redis,
        challenge_id,
        auth_state_id,
        principal_id,
        "webauthn_login",
    )
    .await?;

    let state_value = challenge
        .metadata
        .get("authentication")
        .cloned()
        .ok_or_else(|| {
            AppError::internal("webauthn_state_missing", "Authentication state is missing.")
        })?;
    let stored: StoredPasskeyAuthentication =
        serde_json::from_value(state_value).map_err(|_| {
            AppError::internal("webauthn_state_invalid", "Authentication state is invalid.")
        })?;

    let mut passkeys = load_passkeys(db, principal_id).await?;
    let result = webauthn
        .finish_passkey_authentication(credential, &stored.authentication)
        .map_err(map_webauthn_authentication_error)?;

    let signals = record_passkey_authentication(db, principal_id, &mut passkeys, &result).await?;

    login_challenges::consume_challenge(
        redis,
        challenge.id,
        challenge.auth_state_id,
        challenge.principal_id.ok_or_else(|| {
            AppError::internal(
                "challenge_principal_missing",
                "Login challenge is missing principal context.",
            )
        })?,
        "webauthn_login",
    )
    .await?;

    let geo_signal = risk::geo::apply_geo_security_signal(
        db,
        config,
        principal_id,
        ip,
        0.0,
        json!({
            "cred_id": format!("{:?}", result.cred_id()),
            "auth_state_id": auth_state_id,
            "assurance": signals.assurance.as_str(),
            "backup_eligible": signals.backup_eligible,
            "backup_state": signals.backup_state,
            "sign_count": signals.sign_count,
        }),
        "webauthn_login_authenticated",
    )
    .await;
    let _ = risk::record_event(
        db,
        RiskEventInput {
            principal_id,
            session_id: None,
            device_id: None,
            event_type: "webauthn_login_authenticated".to_string(),
            ip_address: ip.map(ToOwned::to_owned),
            user_agent: user_agent.map(ToOwned::to_owned),
            risk_score: geo_signal.score,
            risk_factors: geo_signal.factors,
            decision: geo_signal.decision,
            metadata: json!({ "geo": geo_signal.metadata }),
        },
    )
    .await;

    Ok("webauthn".to_string())
}

fn shape_authentication_options(mut options: serde_json::Value) -> serde_json::Value {
    if let Some(public_key) = options.get_mut("publicKey")
        && let Some(public_key_object) = public_key.as_object_mut()
    {
        public_key_object.insert(
            "hints".to_string(),
            serde_json::json!(["security-key", "client-device", "hybrid"]),
        );
    }
    options
}
