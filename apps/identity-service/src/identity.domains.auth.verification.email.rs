use chrono::{Duration, Utc};
use rand::Rng;
use uuid::Uuid;

use crate::domains::auth::types::{EmailStepUpChallengeResult, StepUpPurpose};
use crate::domains::auth::{db, token_hash};
use crate::http::error::AppError;

const EMAIL_CODE_TTL_MINUTES: i64 = 10;

pub async fn request_password_change_code(
    db_pool: &sqlx::PgPool,
    redis: &nvbes_redis::RedisPool,
    user_id: Uuid,
    session_id: Uuid,
    purpose: StepUpPurpose,
) -> Result<EmailStepUpChallengeResult, AppError> {
    if purpose != StepUpPurpose::PasswordChange {
        return Err(AppError::bad_request(
            "email_step_up_not_allowed",
            "Email verification is only available for password changes.",
        ));
    }

    let user = db::fetch_user_record(db_pool, user_id).await?;
    if user.email_verified_at.is_none() {
        return Err(AppError::forbidden(
            "verified_email_required",
            "A verified email address is required.",
        ));
    }

    let challenge_id = Uuid::new_v4();
    let code = format!("{:06}", rand::rng().random_range(0..1_000_000_u32));
    let expires_at = Utc::now() + Duration::minutes(EMAIL_CODE_TTL_MINUTES);
    nvbes_redis::email_step_up::store_email_step_up_challenge(
        redis,
        &nvbes_redis::email_step_up::CachedEmailStepUpChallenge {
            id: challenge_id,
            principal_id: user_id,
            session_id,
            code_hash: token_hash(&code),
            expires_at,
        },
    )
    .await
    .map_err(|error| AppError::internal("email_step_up_store_failed", error.to_string()))?;

    if let Err(error) = crate::email::commands::enqueue(
        redis,
        user.email,
        Some(user.display_name.clone()),
        format!("password-change-step-up:{challenge_id}"),
        nvbes_email::EmailTemplate::PasswordChangeCodeV1 {
            user_name: user.display_name,
            code,
            credential_expires_at: expires_at,
        },
        expires_at,
        Some(user_id),
    )
    .await
    {
        let _ =
            nvbes_redis::email_step_up::consume_email_step_up_challenge(redis, challenge_id).await;
        return Err(error.into());
    }

    Ok(EmailStepUpChallengeResult {
        challenge_id,
        expires_at,
    })
}

pub(super) async fn verify_password_change_code(
    redis: &nvbes_redis::RedisPool,
    user_id: Uuid,
    session_id: Uuid,
    challenge_id: Uuid,
    code: &str,
) -> Result<(), AppError> {
    let challenge = nvbes_redis::email_step_up::take_email_step_up_challenge(redis, challenge_id)
        .await
        .map_err(|error| AppError::internal("email_step_up_read_failed", error.to_string()))?
        .ok_or_else(|| {
            AppError::unauthorized("email_code_invalid", "The code is invalid or expired.")
        })?;
    if !challenge_matches(
        &challenge,
        user_id,
        session_id,
        &token_hash(code),
        Utc::now(),
    ) {
        return Err(AppError::unauthorized(
            "email_code_invalid",
            "The code is invalid or expired.",
        ));
    }
    Ok(())
}

fn challenge_matches(
    challenge: &nvbes_redis::email_step_up::CachedEmailStepUpChallenge,
    user_id: Uuid,
    session_id: Uuid,
    submitted_code_hash: &str,
    now: chrono::DateTime<Utc>,
) -> bool {
    challenge.principal_id == user_id
        && challenge.session_id == session_id
        && challenge.expires_at > now
        && challenge.code_hash == submitted_code_hash
}

#[cfg(test)]
mod tests {
    use super::challenge_matches;
    use chrono::{Duration, Utc};
    use uuid::Uuid;

    fn challenge(
        user_id: Uuid,
        session_id: Uuid,
    ) -> nvbes_redis::email_step_up::CachedEmailStepUpChallenge {
        nvbes_redis::email_step_up::CachedEmailStepUpChallenge {
            id: Uuid::new_v4(),
            principal_id: user_id,
            session_id,
            code_hash: "expected".to_string(),
            expires_at: Utc::now() + Duration::minutes(10),
        }
    }

    #[test]
    fn email_code_is_bound_to_user_session_and_password_change() {
        let user_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let challenge = challenge(user_id, session_id);
        let now = Utc::now();

        assert!(challenge_matches(
            &challenge, user_id, session_id, "expected", now
        ));
        assert!(!challenge_matches(
            &challenge,
            user_id,
            Uuid::new_v4(),
            "expected",
            now
        ));
        assert!(!challenge_matches(
            &challenge, user_id, session_id, "wrong", now
        ));
    }
}
