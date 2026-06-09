use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;
use webauthn_rs::prelude::*;

use super::super::risk::{self, RiskDecision, RiskEventInput};
use super::storage::{load_passkeys, persist_passkey};
use super::types::StoredPasskeyAuthentication;
use crate::http::error::AppError;

pub async fn start_authentication(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    webauthn: &webauthn_rs::Webauthn,
    session_id: Uuid,
    user_id: Uuid,
    tenant_id: Uuid,
    workspace_id: Option<Uuid>,
) -> Result<(Uuid, serde_json::Value), AppError> {
    let passkeys = load_passkeys(db, user_id).await?;
    if passkeys.is_empty() {
        return Err(AppError::forbidden(
            "webauthn_not_configured",
            "No active WebAuthn credentials are configured.",
        ));
    }
    let (request, authentication) =
        webauthn
            .start_passkey_authentication(&passkeys)
            .map_err(|_| {
                AppError::internal(
                    "webauthn_auth_start_failed",
                    "Failed to start WebAuthn authentication.",
                )
            })?;

    let challenge_id = Uuid::new_v4();
    nvbes_redis::auth_challenge::store_auth_challenge(
        redis,
        &nvbes_redis::auth_challenge::CachedAuthChallenge {
            id: challenge_id,
            principal_id: user_id,
            session_id,
            tenant_id,
            workspace_id,
            purpose: "webauthn_step_up".to_string(),
            required_level: "aal2".to_string(),
            allowed_factor_types: vec!["webauthn".to_string()],
            factor_id: None,
            metadata: json!({
                "authentication": StoredPasskeyAuthentication { authentication },
            }),
            expires_at: chrono::Utc::now() + chrono::Duration::minutes(5),
        },
    )
    .await
    .map_err(|err| {
        AppError::internal(
            "webauthn_challenge_store_failed",
            format!("Failed to store WebAuthn challenge: {err}"),
        )
    })?;

    let _ = risk::record_event(
        db,
        RiskEventInput {
            principal_id: user_id,
            session_id: Some(session_id),
            device_id: None,
            event_type: "webauthn_auth_started".to_string(),
            ip_address: None,
            user_agent: None,
            risk_score: 5.0,
            risk_factors: json!({
                "passkey_count": passkeys.len(),
            }),
            decision: RiskDecision::Allow,
            metadata: json!({}),
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

#[expect(
    clippy::too_many_arguments,
    reason = "WebAuthn finish keeps challenge ownership and verifier inputs explicit."
)]
pub async fn finish_authentication(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    webauthn: &webauthn_rs::Webauthn,
    session_id: Uuid,
    user_id: Uuid,
    tenant_id: Uuid,
    workspace_id: Option<Uuid>,
    challenge_id: Uuid,
    credential: &PublicKeyCredential,
) -> Result<String, AppError> {
    let challenge = nvbes_redis::auth_challenge::get_auth_challenge(redis, challenge_id)
        .await
        .map_err(|err| AppError::internal("webauthn_challenge_load_failed", format!("{}", err)))?
        .ok_or_else(|| AppError::not_found("challenge_not_found", "Challenge not found."))?;

    if challenge.principal_id != user_id
        || challenge.session_id != session_id
        || challenge.tenant_id != tenant_id
        || challenge.workspace_id != workspace_id
        || challenge.purpose != "webauthn_step_up"
        || challenge.expires_at <= chrono::Utc::now()
    {
        return Err(AppError::not_found(
            "challenge_not_found",
            "Challenge not found.",
        ));
    }

    let metadata = challenge.metadata;
    let state_value = metadata.get("authentication").cloned().ok_or_else(|| {
        AppError::internal("webauthn_state_missing", "Authentication state is missing.")
    })?;
    let stored: StoredPasskeyAuthentication =
        serde_json::from_value(state_value).map_err(|_| {
            AppError::internal("webauthn_state_invalid", "Authentication state is invalid.")
        })?;

    let mut passkeys = load_passkeys(db, user_id).await?;
    let result = webauthn
        .finish_passkey_authentication(credential, &stored.authentication)
        .map_err(|_| AppError::forbidden("webauthn_auth_failed", "WebAuthn assertion failed."))?;

    if let Some(passkey) = passkeys
        .iter_mut()
        .find(|passkey| passkey.cred_id() == result.cred_id())
    {
        passkey.update_credential(&result);
        persist_passkey(db, user_id, passkey).await?;
    }

    nvbes_redis::auth_challenge::consume_auth_challenge(redis, challenge_id)
        .await
        .map_err(|err| {
            AppError::internal("webauthn_challenge_consume_failed", format!("{}", err))
        })?;

    let _ = risk::record_event(
        db,
        RiskEventInput {
            principal_id: user_id,
            session_id: Some(session_id),
            device_id: None,
            event_type: "webauthn_authenticated".to_string(),
            ip_address: None,
            user_agent: None,
            risk_score: 0.0,
            risk_factors: json!({
                "cred_id": format!("{:?}", result.cred_id()),
            }),
            decision: RiskDecision::Allow,
            metadata: json!({}),
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
            json!(["security-key", "client-device", "hybrid"]),
        );
    }
    options
}
