use chrono::{Duration, Utc};
use nvbes_core::config::AppConfig;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;
use webauthn_rs::prelude::PublicKeyCredential;

use super::super::super::login_challenges;
use super::super::super::risk::{self, RiskDecision, RiskEventInput};
use super::super::storage::{discoverable_keys, fetch_user_email, load_passkeys, persist_passkey};
use super::super::types::StoredDiscoverableAuthentication;
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

    Ok((challenge_id, super::shape_authentication_options(options)))
}

pub async fn finish_discoverable_login_authentication(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    config: &AppConfig,
    webauthn: &webauthn_rs::Webauthn,
    challenge_id: Uuid,
    credential: &PublicKeyCredential,
    ip: Option<&str>,
    user_agent: Option<&str>,
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

    let geo_signal = risk::geo::apply_geo_security_signal(
        db,
        config,
        principal_id,
        ip,
        0.0,
        json!({
            "cred_id": format!("{:?}", result.cred_id()),
            "challenge_id": challenge_id,
        }),
        "webauthn_discoverable_login_authenticated",
    )
    .await;
    let _ = risk::record_event(
        db,
        RiskEventInput {
            principal_id,
            session_id: None,
            device_id: None,
            event_type: "webauthn_discoverable_login_authenticated".to_string(),
            ip_address: ip.map(ToOwned::to_owned),
            user_agent: user_agent.map(ToOwned::to_owned),
            risk_score: geo_signal.score,
            risk_factors: geo_signal.factors,
            decision: RiskDecision::Allow,
            metadata: json!({ "geo": geo_signal.metadata }),
        },
    )
    .await;

    Ok((principal_id, email, "webauthn".to_string()))
}
