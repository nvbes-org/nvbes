use nvbes_core::auth::Aal;
use sqlx::PgPool;
use webauthn_rs::prelude::PublicKeyCredential;

use crate::domains::auth::verify_password;
use crate::domains::auth::{db, mfa, types::StepUpInput, webauthn};
use crate::http::error::AppError;

pub(super) struct ResolvedStepUpMethod {
    pub authenticated_method: String,
    pub level: Aal,
}

pub(super) async fn resolve_step_up_method(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    webauthn_instance: &webauthn_rs::Webauthn,
    auth: &impl crate::domains::auth::types::StepUpSubject,
    input: StepUpInput,
) -> Result<ResolvedStepUpMethod, AppError> {
    if let Some(code) = input.totp_code {
        mfa::verify_totp(db, auth.user_id(), &code).await?;
        return Ok(ResolvedStepUpMethod {
            authenticated_method: "otp".to_string(),
            level: Aal::Aal2,
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
            level: Aal::Aal2,
        });
    }

    if let Some(password) = input.password {
        let user = db::fetch_user_record(db, auth.user_id()).await?;
        verify_password(
            user.password_hash.as_deref().ok_or_else(|| {
                AppError::forbidden(
                    "password_missing",
                    "No password is configured for this account.",
                )
            })?,
            &password,
        )?;
        return Ok(ResolvedStepUpMethod {
            authenticated_method: "pwd".to_string(),
            level: Aal::Aal2,
        });
    }

    Err(AppError::bad_request(
        "validation_failed",
        "A password, TOTP code, recovery code, or WebAuthn assertion is required.",
    ))
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

    let method = webauthn::finish_authentication(
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
        authenticated_method: method,
        level: Aal::Aal3,
    })
}
