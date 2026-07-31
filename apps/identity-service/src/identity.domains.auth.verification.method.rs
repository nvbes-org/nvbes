use nvbes_core::auth::Aal;
use sqlx::PgPool;
use webauthn_rs::prelude::PublicKeyCredential;

use crate::domains::auth::verify_password_with_pepper;
use crate::domains::auth::{
    db, mfa,
    types::{StepUpInput, StepUpPurpose},
    webauthn,
};
use crate::http::error::AppError;

pub(super) struct ResolvedStepUpMethod {
    pub authenticated_method: String,
    pub level: Aal,
    pub apply_to_session: bool,
}

pub(super) async fn resolve_step_up_method(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    webauthn_instance: &webauthn_rs::Webauthn,
    config: &nvbes_core::config::AppConfig,
    auth: &impl crate::domains::auth::types::StepUpSubject,
    input: StepUpInput,
) -> Result<ResolvedStepUpMethod, AppError> {
    if let (Some(code), Some(challenge_id), Some(StepUpPurpose::PasswordChange)) = (
        input.email_code.as_deref(),
        input.email_challenge_id,
        input.purpose,
    ) {
        super::email::verify_password_change_code(
            redis,
            auth.user_id(),
            auth.session_id(),
            challenge_id,
            code,
        )
        .await?;
        return Ok(ResolvedStepUpMethod {
            authenticated_method: "email".to_string(),
            level: Aal::Aal1,
            apply_to_session: false,
        });
    }

    if input.email_code.is_some() || input.email_challenge_id.is_some() {
        return Err(AppError::bad_request(
            "email_step_up_not_allowed",
            "Email verification is only available for password changes.",
        ));
    }

    if let Some(code) = input.totp_code {
        mfa::verify_totp(db, config, auth.user_id(), &code).await?;
        return Ok(ResolvedStepUpMethod {
            authenticated_method: "otp".to_string(),
            level: Aal::Aal2,
            apply_to_session: true,
        });
    }

    if let Some(credential) = input.webauthn_response.as_ref() {
        return resolve_webauthn_method(
            db,
            redis,
            webauthn_instance,
            auth,
            input.webauthn_challenge_id,
            credential,
        )
        .await;
    }

    if let Some(code) = input.recovery_code {
        mfa::verify_recovery(db, auth.user_id(), &code).await?;
        return Ok(ResolvedStepUpMethod {
            authenticated_method: "recovery".to_string(),
            level: Aal::Aal1,
            apply_to_session: true,
        });
    }

    if let Some(password) = input.password {
        ensure_password_step_up_allowed(mfa::has_active_factor(db, auth.user_id()).await?)?;
        let user = db::fetch_user_record(db, auth.user_id()).await?;
        verify_password_with_pepper(
            user.password_hash.as_deref().ok_or_else(|| {
                AppError::forbidden(
                    "password_missing",
                    "No password is configured for this account.",
                )
            })?,
            &password,
            config.auth_password_pepper.as_deref(),
        )?;
        return Ok(ResolvedStepUpMethod {
            authenticated_method: "pwd".to_string(),
            level: Aal::Aal1,
            apply_to_session: true,
        });
    }

    Err(AppError::bad_request(
        "validation_failed",
        "A password, TOTP code, recovery code, or WebAuthn assertion is required.",
    ))
}

fn ensure_password_step_up_allowed(mfa_enabled: bool) -> Result<(), AppError> {
    if mfa_enabled {
        return Err(AppError::forbidden(
            "password_step_up_disabled",
            "Password step-up is disabled while multi-factor authentication is active.",
        ));
    }
    Ok(())
}

async fn resolve_webauthn_method(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    webauthn_instance: &webauthn_rs::Webauthn,
    auth: &impl crate::domains::auth::types::StepUpSubject,
    challenge_id: Option<uuid::Uuid>,
    credential: &PublicKeyCredential,
) -> Result<ResolvedStepUpMethod, AppError> {
    let challenge_id = challenge_id.ok_or_else(|| {
        AppError::bad_request("validation_failed", "A WebAuthn challenge id is required.")
    })?;
    let tenant_id = auth.tenant_id().ok_or_else(|| {
        AppError::internal(
            "tenant_context_missing",
            "Authenticated session is missing tenant context.",
        )
    })?;

    let authentication = webauthn::finish_authentication(
        db,
        redis,
        webauthn_instance,
        auth.session_id(),
        auth.user_id(),
        tenant_id,
        auth.workspace_id(),
        challenge_id,
        credential,
    )
    .await?;

    Ok(ResolvedStepUpMethod {
        authenticated_method: "webauthn".to_string(),
        level: authentication.assurance.aal(),
        apply_to_session: true,
    })
}

#[cfg(test)]
mod tests {
    use super::ensure_password_step_up_allowed;

    #[test]
    fn password_step_up_is_rejected_when_mfa_is_active() {
        let error = ensure_password_step_up_allowed(true).expect_err("MFA must disable password");
        assert_eq!(error.code, "password_step_up_disabled");
    }

    #[test]
    fn password_step_up_remains_available_without_mfa() {
        assert!(ensure_password_step_up_allowed(false).is_ok());
    }
}
