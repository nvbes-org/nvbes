use chrono::Utc;
use nvbes_core::auth::Aal;
use sqlx::PgPool;

use super::types::*;
use crate::domains::auth::sessions::cache::{apply_step_up, current_session_ttl};
use crate::http::error::AppError;

#[path = "identity.domains.auth.verification.method.rs"]
mod method;

pub async fn step_up(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    auth_step_up_ttl_minutes: i64,
    webauthn: &webauthn_rs::Webauthn,
    auth: &impl StepUpSubject,
    input: StepUpInput,
) -> Result<StepUpResult, AppError> {
    let resolved = method::resolve_step_up_method(db, redis, webauthn, auth, input).await?;

    let now = Utc::now();
    let valid_until = now + chrono::Duration::minutes(auth_step_up_ttl_minutes);
    if let Ok(Some(mut session)) =
        nvbes_redis::session::get_session(redis, &auth.session_id().to_string()).await
    {
        apply_step_up(
            &mut session,
            resolved.level.as_str(),
            vec![resolved.authenticated_method.clone()],
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
