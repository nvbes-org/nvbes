use chrono::{Duration as ChronoDuration, Utc};
use nvbes_core::config::AppConfig;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::password::history;
use super::{
    db, generate_random_token, hash_password, log_dev_token, normalize_email, token_hash, types::*,
    validate_email, validate_password,
};
use crate::http::error::AppError;

pub async fn register(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    config: &AppConfig,
    input: RegisterInput,
) -> Result<RegisterResult, AppError> {
    let email = normalize_email(&input.email);
    validate_email(&email)?;
    validate_password(&input.password)?;

    if let Some(bd) = input.birthdate {
        nvbes_core::auth::helpers::validate_birthdate(bd, input.region.as_deref())?;
    }

    nvbes_core::limiter::check_rate_limit(
        redis,
        "register",
        &email,
        6,
        std::time::Duration::from_secs(300),
    )
    .await?;

    let password_hash = hash_password(&input.password)?;
    let verification_token = generate_random_token();
    log_dev_token(
        &verification_token,
        &config.environment,
        "email_verification",
    );

    let (principal_id, workspace_id, now) = db::create_user_account(
        db,
        redis,
        config,
        email.clone(),
        input.firstname.clone(),
        input.lastname.clone(),
        input.username.clone(),
        input.birthdate,
        input.region.clone(),
        input.data_region.clone(),
        input.workspace_name.clone(),
        password_hash.clone(),
        verification_token.clone(),
        input.ip.clone(),
        input.user_agent.clone(),
    )
    .await?;
    history::insert_password_hash(db, principal_id, &password_hash).await?;
    let display_name = derive_display_name(
        input.firstname.as_deref(),
        input.lastname.as_deref(),
        Some(&input.username),
    );

    Ok(RegisterResult {
        user: UserView {
            id: principal_id,
            email: email.clone(),
            display_name,
            firstname: input.firstname.clone(),
            lastname: input.lastname.clone(),
            username: Some(input.username.clone()),
            birthdate: input.birthdate,
            region: input.region.clone(),
            email_verified: false,
            mfa_enabled: false,
            created_at: now,
        },
        workspace: WorkspaceView {
            id: workspace_id,
            owner_principal_id: principal_id,
            name: input.workspace_name,
            workspace_type: "personal".to_string(),
            data_region: input
                .data_region
                .clone()
                .unwrap_or_else(|| "eu".to_string()),
            role: "owner".to_string(),
            trial_ends_at: Some(now + ChronoDuration::days(14)),
        },
        verification_resend_available_at:
            super::email_verification::verification_resend_available_at(
                now,
                config.auth_verification_resend_cooldown_seconds,
            ),
    })
}

pub async fn verify_email(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    input: VerifyEmailInput,
) -> Result<VerifyEmailResult, AppError> {
    let token_hash = token_hash(&input.token);
    let row = nvbes_redis::email_verification::get_email_verification_token(redis, &token_hash)
        .await
        .map_err(|err| {
            AppError::internal("email_verification_token_read_failed", &err.to_string())
        })?
        .ok_or_else(|| {
            AppError::not_found(
                "verification_token_not_found",
                "Invalid verification token.",
            )
        })?;

    let principal_id: Uuid = row.principal_id;
    let expires_at: chrono::DateTime<Utc> = row.expires_at;
    let consumed_at: Option<chrono::DateTime<Utc>> = row.consumed_at;
    if consumed_at.is_some() || expires_at <= Utc::now() {
        return Err(AppError::forbidden(
            "verification_token_expired",
            "Verification token is expired or already used.",
        ));
    }

    nvbes_redis::email_verification::mark_email_verification_token_consumed(redis, &token_hash)
        .await
        .map_err(|err| {
            AppError::internal("email_verification_token_consume_failed", &err.to_string())
        })?;
    sqlx::query("UPDATE users SET email_verified_at = NOW(), status = 'active', updated_at = NOW() WHERE principal_id = $1")
        .bind(principal_id)
        .execute(db)
        .await?;
    sqlx::query("UPDATE principals SET status = 'active', updated_at = NOW() WHERE id = $1")
        .bind(principal_id)
        .execute(db)
        .await?;

    let user_row = sqlx::query(
        "SELECT principal_id, email, firstname, lastname, username, birthdate, region, created_at, email_verified_at FROM users WHERE principal_id = $1"
    )
    .bind(principal_id)
    .fetch_one(db)
    .await?;

    let firstname: Option<String> = user_row.get("firstname");
    let lastname: Option<String> = user_row.get("lastname");
    let username: Option<String> = user_row.get("username");
    let display_name = derive_display_name(
        firstname.as_deref(),
        lastname.as_deref(),
        username.as_deref(),
    );
    Ok(VerifyEmailResult {
        success: true,
        user: UserView {
            id: user_row.get("principal_id"),
            email: user_row.get("email"),
            display_name,
            firstname,
            lastname,
            username,
            birthdate: user_row.get("birthdate"),
            region: user_row.get("region"),
            email_verified: true,
            mfa_enabled: false,
            created_at: user_row.get("created_at"),
        },
    })
}
