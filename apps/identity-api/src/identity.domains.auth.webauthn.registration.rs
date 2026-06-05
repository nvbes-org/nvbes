use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;
use webauthn_rs::prelude::*;

use super::super::risk::{self, RiskDecision, RiskEventInput};
use super::errors::map_webauthn_registration_error;
use super::storage::load_passkeys;
use super::types::StoredWebauthnRegistration;
use crate::http::error::AppError;

pub async fn start_registration(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    webauthn: &webauthn_rs::Webauthn,
    session_id: Uuid,
    user_id: Uuid,
    tenant_id: Uuid,
    workspace_id: Option<Uuid>,
    label: Option<String>,
    kind: Option<String>,
) -> Result<(Uuid, serde_json::Value), AppError> {
    let kind = normalize_registration_kind(kind)?;
    let existing = load_passkeys(db, user_id).await?;
    let exclude_credentials = existing
        .iter()
        .map(|passkey| passkey.cred_id().clone())
        .collect::<Vec<_>>();
    let (creation, registration) = if kind == "security_key" {
        let (creation, registration) = webauthn
            .start_securitykey_registration(
                user_id,
                &user_id.to_string(),
                label.as_deref().unwrap_or("Passkey"),
                Some(exclude_credentials),
                None,
                Some(AuthenticatorAttachment::CrossPlatform),
            )
            .map_err(|_| {
                AppError::internal(
                    "webauthn_registration_start_failed",
                    "Failed to start WebAuthn registration.",
                )
            })?;
        (
            creation,
            StoredWebauthnRegistration::SecurityKey { registration },
        )
    } else {
        let (creation, registration) = webauthn
            .start_passkey_registration(
                user_id,
                &user_id.to_string(),
                label.as_deref().unwrap_or("Passkey"),
                Some(exclude_credentials),
            )
            .map_err(|_| {
                AppError::internal(
                    "webauthn_registration_start_failed",
                    "Failed to start WebAuthn registration.",
                )
            })?;
        (
            creation,
            StoredWebauthnRegistration::Passkey { registration },
        )
    };

    let factor_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO mfa_factors (
          id, principal_id, factor_type, status, label, factor_data, created_at
        )
        VALUES ($1, $2, 'webauthn', 'pending', $3, $4, NOW())
        "#,
    )
    .bind(factor_id)
    .bind(user_id)
    .bind(label.clone().unwrap_or_else(|| "Passkey".to_string()))
    .bind(sqlx::types::Json(json!({
        "registration": registration.clone(),
        "session_id": session_id,
        "kind": kind,
    })))
    .execute(db)
    .await?;

    let challenge_id = factor_id;
    nvbes_redis::auth_challenge::store_auth_challenge(
        redis,
        &nvbes_redis::auth_challenge::CachedAuthChallenge {
            id: challenge_id,
            principal_id: user_id,
            session_id,
            tenant_id,
            workspace_id,
            purpose: "webauthn_registration".to_string(),
            required_level: "aal2".to_string(),
            allowed_factor_types: vec!["webauthn".to_string()],
            factor_id: Some(factor_id),
            metadata: json!({
                "factor_id": factor_id,
                "label": label,
                "kind": kind,
                "registration": registration,
            }),
            expires_at: chrono::Utc::now() + chrono::Duration::minutes(5),
        },
    )
    .await
    .map_err(|err| {
        AppError::internal(
            "webauthn_challenge_store_failed",
            &format!("Failed to store WebAuthn challenge: {err}"),
        )
    })?;

    let _ = risk::record_event(
        db,
        RiskEventInput {
            principal_id: user_id,
            session_id: Some(session_id),
            device_id: None,
            event_type: "webauthn_registration_started".to_string(),
            ip_address: None,
            user_agent: None,
            risk_score: 10.0,
            risk_factors: json!({
                "existing_credentials": existing.len(),
            }),
            decision: RiskDecision::Allow,
            metadata: json!({
                "label": label,
            }),
        },
    )
    .await;

    let options = serde_json::to_value(creation).map_err(|_| {
        AppError::internal(
            "webauthn_serialization_failed",
            "Failed to serialize WebAuthn options.",
        )
    })?;
    let options = shape_registration_options(options, kind);

    Ok((challenge_id, options))
}

pub async fn finish_registration(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    webauthn: &webauthn_rs::Webauthn,
    session_id: Uuid,
    user_id: Uuid,
    factor_id: Uuid,
    reg: RegisterPublicKeyCredential,
) -> Result<(), AppError> {
    let challenge = nvbes_redis::auth_challenge::get_auth_challenge(redis, factor_id)
        .await
        .map_err(|err| AppError::internal("webauthn_challenge_load_failed", &format!("{}", err)))?
        .ok_or_else(|| AppError::not_found("challenge_not_found", "Challenge not found."))?;

    if challenge.principal_id != user_id
        || challenge.session_id != session_id
        || challenge.purpose != "webauthn_registration"
        || challenge.expires_at <= chrono::Utc::now()
    {
        return Err(AppError::not_found(
            "challenge_not_found",
            "Challenge not found.",
        ));
    }

    let metadata = challenge.metadata;
    let state_value = metadata.get("registration").cloned().ok_or_else(|| {
        AppError::internal("webauthn_state_missing", "Registration state is missing.")
    })?;
    let stored: StoredWebauthnRegistration = serde_json::from_value(state_value).map_err(|_| {
        AppError::internal("webauthn_state_invalid", "Registration state is invalid.")
    })?;
    let kind = metadata
        .get("kind")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("passkey");

    let credential = match stored {
        StoredWebauthnRegistration::Passkey { registration } => serde_json::to_value(
            webauthn
                .finish_passkey_registration(&reg, &registration)
                .map_err(map_webauthn_registration_error)?,
        )
        .map_err(|_| {
            AppError::internal(
                "webauthn_registration_serialization_failed",
                "Failed to serialize WebAuthn credential.",
            )
        })?,
        StoredWebauthnRegistration::SecurityKey { registration } => serde_json::to_value(
            webauthn
                .finish_securitykey_registration(&reg, &registration)
                .map_err(map_webauthn_registration_error)?,
        )
        .map_err(|_| {
            AppError::internal(
                "webauthn_registration_serialization_failed",
                "Failed to serialize WebAuthn credential.",
            )
        })?,
    };

    sqlx::query(
        r#"
        UPDATE mfa_factors
        SET status = 'active',
            confirmed_at = COALESCE(confirmed_at, NOW()),
            factor_data = jsonb_build_object(
              'passkey',
              $3::jsonb,
              'kind',
              $4::text
            )
        WHERE id = $1 AND principal_id = $2 AND factor_type = 'webauthn'
        "#,
    )
    .bind(factor_id)
    .bind(user_id)
    .bind(sqlx::types::Json(credential))
    .bind(kind)
    .execute(db)
    .await?;

    nvbes_redis::auth_challenge::consume_auth_challenge(redis, factor_id)
        .await
        .map_err(|err| {
            AppError::internal("webauthn_challenge_consume_failed", &format!("{}", err))
        })?;

    let _ = risk::record_event(
        db,
        RiskEventInput {
            principal_id: user_id,
            session_id: Some(session_id),
            device_id: None,
            event_type: "webauthn_registered".to_string(),
            ip_address: None,
            user_agent: None,
            risk_score: 5.0,
            risk_factors: json!({
                "factor_id": factor_id,
            }),
            decision: RiskDecision::Allow,
            metadata: json!({}),
        },
    )
    .await;

    Ok(())
}

fn normalize_registration_kind(kind: Option<String>) -> Result<&'static str, AppError> {
    match kind.as_deref().unwrap_or("passkey") {
        "passkey" => Ok("passkey"),
        "security_key" => Ok("security_key"),
        _ => Err(AppError::bad_request(
            "webauthn_registration_kind_invalid",
            "WebAuthn registration kind is invalid.",
        )),
    }
}

fn shape_registration_options(
    mut options: serde_json::Value,
    kind: &'static str,
) -> serde_json::Value {
    let Some(public_key) = options.get_mut("publicKey") else {
        return options;
    };
    if let Some(public_key_object) = public_key.as_object_mut() {
        public_key_object.remove("extensions");
        public_key_object.insert(
            "authenticatorSelection".to_string(),
            match kind {
                "security_key" => json!({
                    "authenticatorAttachment": "cross-platform",
                    "requireResidentKey": false,
                    "residentKey": "discouraged",
                    "userVerification": "preferred",
                }),
                _ => json!({
                    "authenticatorAttachment": "platform",
                    "requireResidentKey": false,
                    "residentKey": "preferred",
                    "userVerification": "required",
                }),
            },
        );
        public_key_object.insert(
            "hints".to_string(),
            json!(["security-key", "client-device", "hybrid"]),
        );
    }

    options
}

#[cfg(test)]
#[path = "identity.domains.auth.webauthn.registration.tests.rs"]
mod tests;
