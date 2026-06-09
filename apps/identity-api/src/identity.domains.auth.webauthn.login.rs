use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;
use webauthn_rs::prelude::PublicKeyCredential;

use super::super::login_challenges::{self, CreateLoginChallengeInput};
use super::super::risk::{self, RiskDecision, RiskEventInput};
use super::storage::{discoverable_keys, fetch_user_email, load_passkeys, persist_passkey};
use super::types::{StoredDiscoverableAuthentication, StoredPasskeyAuthentication};
use crate::http::error::AppError;

#[derive(Debug, Serialize, Deserialize)]
struct CachedDiscoverableLoginChallenge {
    id: Uuid,
    authentication: StoredDiscoverableAuthentication,
    failed_attempts: i32,
    expires_at: chrono::DateTime<Utc>,
}

fn discoverable_login_key(challenge_id: Uuid) -> String {
    format!("nvbes:identity:webauthn-discoverable-login:{challenge_id}")
}

pub async fn start_login_authentication(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    webauthn: &webauthn_rs::Webauthn,
    auth_state_id: Uuid,
    principal_id: Uuid,
) -> Result<(Uuid, serde_json::Value), AppError> {
    let passkeys = load_passkeys(db, principal_id).await?;
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

    let _ = risk::record_event(
        db,
        RiskEventInput {
            principal_id,
            session_id: None,
            device_id: None,
            event_type: "webauthn_login_started".to_string(),
            ip_address: None,
            user_agent: None,
            risk_score: 5.0,
            risk_factors: json!({
                "passkey_count": passkeys.len(),
                "auth_state_id": auth_state_id,
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

pub async fn start_discoverable_login_authentication(
    redis: &nvbes_redis::RedisPool,
    webauthn: &webauthn_rs::Webauthn,
) -> Result<(Uuid, serde_json::Value), AppError> {
    let (request, authentication) = webauthn.start_discoverable_authentication().map_err(|_| {
        AppError::internal(
            "webauthn_auth_start_failed",
            "Failed to start WebAuthn authentication.",
        )
    })?;

    let challenge_id = Uuid::new_v4();
    let expires_at = Utc::now() + Duration::minutes(5);
    let challenge = CachedDiscoverableLoginChallenge {
        id: challenge_id,
        authentication: StoredDiscoverableAuthentication { authentication },
        failed_attempts: 0,
        expires_at,
    };

    nvbes_redis::cache::cache_set_json(
        redis,
        &discoverable_login_key(challenge_id),
        &challenge,
        5 * 60,
    )
    .await
    .map_err(|err| AppError::internal("webauthn_challenge_store_failed", err.to_string()))?;

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
    webauthn: &webauthn_rs::Webauthn,
    auth_state_id: Uuid,
    principal_id: Uuid,
    challenge_id: Uuid,
    credential: &PublicKeyCredential,
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
        .map_err(|_| AppError::forbidden("webauthn_auth_failed", "WebAuthn assertion failed."))?;

    if let Some(passkey) = passkeys
        .iter_mut()
        .find(|passkey| passkey.cred_id() == result.cred_id())
    {
        passkey.update_credential(&result);
        persist_passkey(db, principal_id, passkey).await?;
    }

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

    let _ = risk::record_event(
        db,
        RiskEventInput {
            principal_id,
            session_id: None,
            device_id: None,
            event_type: "webauthn_login_authenticated".to_string(),
            ip_address: None,
            user_agent: None,
            risk_score: 0.0,
            risk_factors: json!({
                "cred_id": format!("{:?}", result.cred_id()),
                "auth_state_id": auth_state_id,
            }),
            decision: RiskDecision::Allow,
            metadata: json!({}),
        },
    )
    .await;

    Ok("webauthn".to_string())
}

pub async fn finish_discoverable_login_authentication(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    webauthn: &webauthn_rs::Webauthn,
    challenge_id: Uuid,
    credential: &PublicKeyCredential,
) -> Result<(Uuid, String, String), AppError> {
    let mut challenge = nvbes_redis::cache::cache_get_json::<CachedDiscoverableLoginChallenge>(
        redis,
        &discoverable_login_key(challenge_id),
    )
    .await
    .map_err(|err| AppError::internal("webauthn_challenge_load_failed", err.to_string()))?
    .ok_or_else(|| AppError::not_found("challenge_not_found", "Challenge not found."))?;

    if challenge.id != challenge_id
        || challenge.expires_at <= Utc::now()
        || challenge.failed_attempts >= login_challenges::MAX_FAILED_ATTEMPTS
    {
        return Err(AppError::not_found(
            "challenge_not_found",
            "Challenge not found.",
        ));
    }

    let (principal_id, _credential_id) = webauthn
        .identify_discoverable_authentication(credential)
        .map_err(|_| AppError::forbidden("webauthn_auth_failed", "WebAuthn assertion failed."))?;
    let mut passkeys = load_passkeys(db, principal_id).await?;
    let discoverable = discoverable_keys(&passkeys);
    let result = match webauthn.finish_discoverable_authentication(
        credential,
        challenge.authentication.authentication.clone(),
        &discoverable,
    ) {
        Ok(result) => result,
        Err(_) => {
            challenge.failed_attempts += 1;
            nvbes_redis::cache::cache_set_json(
                redis,
                &discoverable_login_key(challenge_id),
                &challenge,
                5 * 60,
            )
            .await
            .map_err(|cache_err| {
                AppError::internal("webauthn_challenge_store_failed", cache_err.to_string())
            })?;
            return Err(AppError::forbidden(
                "webauthn_auth_failed",
                "WebAuthn assertion failed.",
            ));
        }
    };

    if let Some(passkey) = passkeys
        .iter_mut()
        .find(|passkey| passkey.cred_id() == result.cred_id())
    {
        passkey.update_credential(&result);
        persist_passkey(db, principal_id, passkey).await?;
    }

    nvbes_redis::RedisClient::new(redis.clone())
        .del_key(&discoverable_login_key(challenge_id))
        .await
        .map_err(|err| AppError::internal("webauthn_challenge_consume_failed", err.to_string()))?;

    let email = fetch_user_email(db, principal_id).await?;

    let _ = risk::record_event(
        db,
        RiskEventInput {
            principal_id,
            session_id: None,
            device_id: None,
            event_type: "webauthn_discoverable_login_authenticated".to_string(),
            ip_address: None,
            user_agent: None,
            risk_score: 0.0,
            risk_factors: json!({
                "cred_id": format!("{:?}", result.cred_id()),
                "challenge_id": challenge_id,
            }),
            decision: RiskDecision::Allow,
            metadata: json!({}),
        },
    )
    .await;

    Ok((principal_id, email, "webauthn".to_string()))
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
