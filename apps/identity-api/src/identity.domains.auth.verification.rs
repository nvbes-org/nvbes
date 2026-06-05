use chrono::Utc;
use nvbes_core::auth::Aal;
use sqlx::PgPool;
use webauthn_rs::prelude::PublicKeyCredential;

use super::{db, mfa, types::*, webauthn};
use crate::domains::auth::sessions::cache::{apply_step_up, current_session_ttl};
use crate::http::error::AppError;

pub async fn step_up(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    auth_step_up_ttl_minutes: i64,
    webauthn: &webauthn_rs::Webauthn,
    auth: &impl StepUpSubject,
    input: StepUpInput,
) -> Result<StepUpResult, AppError> {
    let mut authenticated_method = None;
    let mut level = Aal::Aal1;

    if let Some(code) = input.totp_code {
        mfa::verify_totp(db, auth.user_id(), &code).await?;
        authenticated_method = Some("otp".to_string());
        level = Aal::Aal2;
    } else if input.webauthn_response.is_some() {
        let challenge_id = input.webauthn_challenge_id.ok_or_else(|| {
            AppError::bad_request("validation_failed", "A WebAuthn challenge id is required.")
        })?;
        let tenant_id = auth.tenant_id().ok_or_else(|| {
            AppError::internal(
                "tenant_context_missing",
                "Authenticated session is missing tenant context.",
            )
        })?;
        let credential: &PublicKeyCredential =
            input.webauthn_response.as_ref().ok_or_else(|| {
                AppError::bad_request("validation_failed", "A WebAuthn assertion is required.")
            })?;
        let method = webauthn::finish_authentication(
            db,
            redis,
            webauthn,
            auth.session_id(),
            auth.user_id(),
            tenant_id,
            auth.workspace_id(),
            challenge_id,
            credential,
        )
        .await?;
        authenticated_method = Some(method);
        level = Aal::Aal3;
    } else if let Some(code) = input.recovery_code {
        mfa::verify_recovery(db, auth.user_id(), &code).await?;
        authenticated_method = Some("recovery".to_string());
        level = Aal::Aal2;
    } else if let Some(password) = input.password {
        let user = db::fetch_user_record(db, auth.user_id()).await?;
        super::verify_password(
            user.password_hash.as_deref().ok_or_else(|| {
                AppError::forbidden(
                    "password_missing",
                    "No password is configured for this account.",
                )
            })?,
            &password,
        )?;
        authenticated_method = Some("pwd".to_string());
        level = Aal::Aal2;
    }

    if authenticated_method.is_none() {
        return Err(AppError::bad_request(
            "validation_failed",
            "A password, TOTP code, recovery code, or WebAuthn assertion is required.",
        ));
    }

    let now = Utc::now();
    let valid_until = now + chrono::Duration::minutes(auth_step_up_ttl_minutes);
    let authenticated_method = authenticated_method.unwrap_or_else(|| "pwd".to_string());
    if let Ok(Some(mut session)) =
        nvbes_redis::session::get_session(redis, &auth.session_id().to_string()).await
    {
        apply_step_up(
            &mut session,
            level.as_str(),
            vec![authenticated_method],
            now,
            valid_until,
        );
        let ttl = current_session_ttl(&session);
        if nvbes_redis::session::set_session(redis, &session, ttl)
            .await
            .is_err()
        {
            let _ = nvbes_redis::session::delete_session(
                redis,
                &auth.user_id().to_string(),
                &auth.session_id().to_string(),
            )
            .await;
        }
    }

    Ok(StepUpResult {
        success: true,
        valid_until,
    })
}

pub async fn require_recent_step_up(
    redis: &nvbes_redis::RedisPool,
    auth: &impl StepUpSubject,
    required: Option<Aal>,
) -> Result<(), AppError> {
    let required = required.unwrap_or(Aal::Aal2);
    let session = nvbes_redis::session::get_session(redis, &auth.session_id().to_string())
        .await
        .map_err(|err| AppError::internal("redis_session_read_failed", &err.to_string()))?
        .ok_or_else(|| {
            AppError::unauthorized("step_up_required", "Please verify again before continuing.")
        })?;

    if session.principal_id != auth.user_id().to_string()
        || session.revoked_at.is_some()
        || session.expires_at <= Utc::now()
    {
        return Err(AppError::unauthorized(
            "step_up_required",
            "Please verify again before continuing.",
        ));
    }

    let step_up_verified_at = session.step_up_verified_at;
    let step_up_expires_at = session.step_up_expires_at;
    let current_aal = session
        .acr
        .as_deref()
        .and_then(|acr| acr.parse::<Aal>().ok())
        .unwrap_or(Aal::Aal1);

    if current_aal < required
        || step_up_verified_at.is_none()
        || step_up_expires_at.is_none_or(|value| value <= Utc::now())
    {
        return Err(AppError::unauthorized(
            "step_up_required",
            "Please verify again before continuing.",
        ));
    }

    Ok(())
}
