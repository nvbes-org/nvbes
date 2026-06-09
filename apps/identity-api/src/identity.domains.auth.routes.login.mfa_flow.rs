use uuid::Uuid;
use webauthn_rs::prelude::PublicKeyCredential;

use crate::app::AppConfig;
use crate::database::Database;
use crate::domains::auth::login_challenges;
use crate::http::error::AppError;

use super::types::MfaRequest;

pub(crate) async fn resolve_authenticated_method(
    db: &Database,
    redis: &nvbes_redis::RedisPool,
    config: &AppConfig,
    request: &MfaRequest,
    auth_state_id: Uuid,
    principal_id: Uuid,
) -> Result<String, AppError> {
    validate_single_factor(request)?;

    if let Some(code) = request.totp_code.as_deref() {
        crate::domains::auth::mfa::verify_totp(db, principal_id, code).await?;
        return Ok("otp".to_string());
    }

    if let Some(code) = request.recovery_code.as_deref() {
        crate::domains::auth::mfa::verify_recovery(db, principal_id, code).await?;
        return Ok("recovery".to_string());
    }

    let credential = request.webauthn_response.as_ref().ok_or_else(|| {
        super::validation_failed("A TOTP code, recovery code, or WebAuthn assertion is required.")
    })?;
    resolve_webauthn_method(
        db,
        redis,
        config,
        request,
        auth_state_id,
        principal_id,
        credential,
    )
    .await
}

fn validate_single_factor(request: &MfaRequest) -> Result<(), AppError> {
    let selected_factor_count = usize::from(request.totp_code.is_some())
        + usize::from(request.recovery_code.is_some())
        + usize::from(request.webauthn_response.is_some());
    if selected_factor_count != 1 {
        return Err(super::validation_failed(
            "Exactly one MFA method must be provided.",
        ));
    }
    Ok(())
}

async fn resolve_webauthn_method(
    db: &Database,
    redis: &nvbes_redis::RedisPool,
    config: &AppConfig,
    request: &MfaRequest,
    auth_state_id: Uuid,
    principal_id: Uuid,
    credential: &PublicKeyCredential,
) -> Result<String, AppError> {
    let challenge_id = request
        .webauthn_challenge_id
        .ok_or_else(|| super::validation_failed("A WebAuthn challenge id is required."))?;
    let webauthn = crate::domains::auth::webauthn::build_webauthn(config)?;
    match crate::domains::auth::webauthn::finish_login_authentication(
        db,
        redis,
        &webauthn,
        auth_state_id,
        principal_id,
        challenge_id,
        credential,
    )
    .await
    {
        Ok(method) => Ok(method),
        Err(err) => {
            if let Ok(failed_attempts) = login_challenges::record_failed_attempt(
                redis,
                challenge_id,
                auth_state_id,
                principal_id,
                "webauthn_login",
            )
            .await
                && failed_attempts >= 5
            {
                return Err(super::challenge_locked());
            }
            Err(err)
        }
    }
}
