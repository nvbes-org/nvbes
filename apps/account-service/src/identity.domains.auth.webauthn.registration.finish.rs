use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;
use webauthn_rs::prelude::*;

use crate::domains::auth::{
    risk::{self, RiskDecision, RiskEventInput},
    webauthn::{errors::map_webauthn_registration_error, types::StoredWebauthnRegistration},
};
use crate::http::error::AppError;

pub async fn finish_registration(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    webauthn: &webauthn_rs::Webauthn,
    session_id: Uuid,
    user_id: Uuid,
    factor_id: Uuid,
    reg: RegisterPublicKeyCredential,
) -> Result<(), AppError> {
    let challenge = nvbes_redis::auth_challenge::take_auth_challenge(redis, factor_id)
        .await
        .map_err(|err| AppError::internal("webauthn_challenge_load_failed", format!("{}", err)))?
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
