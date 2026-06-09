use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;
use webauthn_rs::prelude::*;

use super::options::{normalize_registration_kind, shape_registration_options};
use crate::domains::auth::{
    risk::{self, RiskDecision, RiskEventInput},
    webauthn::{storage::load_passkeys, types::StoredWebauthnRegistration},
};
use crate::http::error::AppError;

#[expect(
    clippy::too_many_arguments,
    reason = "WebAuthn registration start keeps challenge ownership and label inputs explicit."
)]
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
            format!("Failed to store WebAuthn challenge: {err}"),
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
