use chrono::Utc;
use nvbes_core::auth::Aal;
use nvbes_core::config::AppConfig;
use sqlx::PgPool;

use super::types::*;
use crate::domains::auth::sessions::cache::{apply_step_up, current_session_ttl};
use crate::http::error::AppError;

#[path = "identity.domains.auth.verification.email.rs"]
pub mod email;
#[path = "identity.domains.auth.verification.grant.rs"]
mod grant;
#[path = "identity.domains.auth.verification.method.rs"]
mod method;

pub async fn step_up(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    config: &AppConfig,
    webauthn: &webauthn_rs::Webauthn,
    auth: &impl StepUpSubject,
    input: StepUpInput,
    rotate_browser_session: bool,
) -> Result<StepUpResult, AppError> {
    let purpose = input.purpose;
    let resolved = method::resolve_step_up_method(db, redis, webauthn, config, auth, input).await?;

    let now = Utc::now();
    let valid_until = now + chrono::Duration::minutes(config.auth_step_up_ttl_minutes);
    if purpose == Some(StepUpPurpose::PasswordChange) {
        grant::store_password_change_grant(redis, auth.user_id(), auth.session_id(), valid_until)
            .await?;
    }
    if !resolved.apply_to_session {
        return Ok(StepUpResult {
            success: true,
            valid_until,
            browser_session_token: None,
        });
    }
    let mut session = nvbes_redis::session::get_session(redis, &auth.session_id().to_string())
        .await
        .map_err(|err| AppError::internal("redis_session_read_failed", err.to_string()))?
        .ok_or_else(nvbes_core::auth::step_up_required_error)?;
    if session.principal_id != auth.user_id().to_string()
        || session.revoked_at.is_some()
        || session.expires_at <= now
    {
        return Err(nvbes_core::auth::step_up_required_error().into());
    }
    apply_step_up(
        &mut session,
        resolved.level.as_str(),
        vec![resolved.authenticated_method.clone()],
        now,
        valid_until,
    );
    let browser_session_token = rotate_browser_session
        .then(|| crate::domains::auth::sessions::token::issue(auth.session_id()));
    if let Some(token) = browser_session_token.as_deref() {
        session.browser_session_token_hash =
            Some(crate::domains::auth::sessions::token::hash(token));
    }
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
        return Err(AppError::internal(
            "session_step_up_update_failed",
            "The session could not be updated after step-up authentication.",
        ));
    }

    Ok(StepUpResult {
        success: true,
        valid_until,
        browser_session_token,
    })
}

pub async fn require_password_change_step_up(
    redis: &nvbes_redis::RedisPool,
    auth: &impl StepUpSubject,
) -> Result<(), AppError> {
    grant::require_password_change_grant(redis, auth.user_id(), auth.session_id()).await
}

pub async fn clear_password_change_step_up(
    redis: &nvbes_redis::RedisPool,
    auth: &impl StepUpSubject,
) -> Result<(), AppError> {
    grant::clear_password_change_grant(redis, auth.user_id(), auth.session_id()).await
}

pub async fn require_recent_account_step_up(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    auth: &impl StepUpSubject,
) -> Result<(), AppError> {
    let required = if crate::domains::auth::mfa::has_active_factor(db, auth.user_id()).await? {
        Aal::Aal2
    } else {
        Aal::Aal1
    };
    require_recent_step_up(redis, auth, Some(required)).await
}

pub async fn require_recent_step_up(
    redis: &nvbes_redis::RedisPool,
    auth: &impl StepUpSubject,
    required: Option<Aal>,
) -> Result<(), AppError> {
    let required = required.unwrap_or(Aal::Aal2);
    let session = nvbes_redis::session::get_session(redis, &auth.session_id().to_string())
        .await
        .map_err(|err| AppError::internal("redis_session_read_failed", err.to_string()))?
        .ok_or_else(|| {
            crate::http::error::AppError::from(nvbes_core::auth::step_up_required_error())
        })?;

    if session.principal_id != auth.user_id().to_string()
        || session.revoked_at.is_some()
        || session.expires_at <= Utc::now()
    {
        return Err(nvbes_core::auth::step_up_required_error().into());
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
        return Err(nvbes_core::auth::step_up_required_error().into());
    }

    Ok(())
}

pub async fn require_recent_phishing_resistant_step_up(
    redis: &nvbes_redis::RedisPool,
    auth: &impl StepUpSubject,
) -> Result<(), AppError> {
    let session = nvbes_redis::session::get_session(redis, &auth.session_id().to_string())
        .await
        .map_err(|err| AppError::internal("redis_session_read_failed", err.to_string()))?
        .ok_or_else(nvbes_core::auth::step_up_required_error)?;
    let current_aal = session
        .acr
        .as_deref()
        .and_then(|acr| acr.parse::<Aal>().ok())
        .unwrap_or(Aal::Aal1);

    if session.principal_id != auth.user_id().to_string()
        || session.revoked_at.is_some()
        || session.expires_at <= Utc::now()
        || current_aal < Aal::Aal2
        || session.step_up_verified_at.is_none()
        || session
            .step_up_expires_at
            .is_none_or(|value| value <= Utc::now())
        || !session.amr.iter().any(|method| method == "webauthn")
    {
        return Err(AppError::forbidden(
            "phishing_resistant_step_up_required",
            "A recent passkey or security-key verification is required.",
        ));
    }

    Ok(())
}

pub async fn require_recent_maximum_assurance_step_up(
    redis: &nvbes_redis::RedisPool,
    auth: &impl StepUpSubject,
) -> Result<(), AppError> {
    require_recent_step_up(redis, auth, Some(Aal::Aal3)).await?;
    require_recent_phishing_resistant_step_up(redis, auth).await
}

pub async fn require_passkey_enrollment_step_up(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    auth: &impl StepUpSubject,
) -> Result<(), AppError> {
    let has_passkey = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
          SELECT 1
          FROM mfa_factors
          WHERE principal_id = $1
            AND factor_type = 'webauthn'
            AND status = 'active'
        )
        "#,
    )
    .bind(auth.user_id())
    .fetch_one(db)
    .await?;

    if has_passkey {
        require_recent_phishing_resistant_step_up(redis, auth).await
    } else {
        require_recent_step_up(redis, auth, Some(Aal::Aal1)).await
    }
}

#[cfg(test)]
mod assurance_tests {
    use nvbes_core::auth::Aal;

    use super::method::ResolvedStepUpMethod;

    #[test]
    fn totp_never_reaches_maximum_assurance() {
        let method = ResolvedStepUpMethod {
            authenticated_method: "otp".to_string(),
            level: Aal::Aal2,
            apply_to_session: true,
        };

        assert_eq!(method.level, Aal::Aal2);
        assert_ne!(method.level, Aal::Aal3);
    }

    #[test]
    fn recovery_is_degraded_assurance() {
        let method = ResolvedStepUpMethod {
            authenticated_method: "recovery".to_string(),
            level: Aal::Aal1,
            apply_to_session: true,
        };

        assert_eq!(method.level, Aal::Aal1);
    }
}
