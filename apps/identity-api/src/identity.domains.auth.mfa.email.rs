use chrono::{DateTime, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::domains::auth::{db, password::token_hash, types::*};
use crate::http::error::AppError;

const EMAIL_MFA_CODE_TTL_SECONDS: u64 = 300;

#[derive(Debug, Serialize, Deserialize)]
struct CachedEmailMfaCode {
    principal_id: Uuid,
    auth_state_id: Uuid,
    code_hash: String,
    created_at: DateTime<Utc>,
}

pub async fn eligible_emails(db: &PgPool, user_id: Uuid) -> Result<EmailAddressesResult, AppError> {
    Ok(EmailAddressesResult {
        emails: db::emails::verified_mfa_eligible_emails(db, user_id).await?,
        primary_min_age_hours: db::emails::primary_email_policy_hours(db, user_id).await?,
    })
}

pub async fn setup_email_factor(
    db: &PgPool,
    user_id: Uuid,
    email_address_id: Uuid,
) -> Result<TotpConfirmResult, AppError> {
    let eligible = db::emails::verified_mfa_eligible_emails(db, user_id).await?;
    let email = eligible
        .into_iter()
        .find(|email| email.id == email_address_id)
        .ok_or_else(|| {
            AppError::forbidden(
                "email_not_mfa_eligible",
                "This email is not eligible for email MFA.",
            )
        })?;
    let factor_id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query(
        r#"
        INSERT INTO mfa_factors (
          id,
          principal_id,
          factor_type,
          status,
          label,
          factor_data,
          created_at,
          confirmed_at
        )
        VALUES ($1, $2, 'email', 'active', $3, $4, $5, $5)
        "#,
    )
    .bind(factor_id)
    .bind(user_id)
    .bind(&email.email)
    .bind(serde_json::json!({
        "email_id": email.id,
        "email": email.email,
    }))
    .bind(now)
    .execute(db)
    .await?;

    Ok(TotpConfirmResult {
        factor: MfaFactorView {
            id: factor_id,
            factor_type: "email".to_string(),
            kind: None,
            status: "active".to_string(),
            label: Some(email.email),
            created_at: now,
            confirmed_at: Some(now),
            last_used_at: None,
        },
        mfa_enabled: true,
    })
}

pub async fn ensure_primary_email_factor(db: &PgPool, user_id: Uuid) -> Result<(), AppError> {
    let has_email_factor = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
          SELECT 1
          FROM mfa_factors
          WHERE principal_id = $1
            AND factor_type = 'email'
            AND status = 'active'
        )
        "#,
    )
    .bind(user_id)
    .fetch_one(db)
    .await?;
    if has_email_factor {
        return Ok(());
    }

    let row = sqlx::query(
        r#"
        SELECT email, email_verified_at
        FROM users
        WHERE principal_id = $1
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::not_found("user_not_found", "User not found."))?;

    let email: String = row.get("email");
    let verified_at: Option<DateTime<Utc>> = row.get("email_verified_at");
    if verified_at.is_none() {
        return Err(AppError::forbidden(
            "email_mfa_requires_verified_email",
            "Email MFA requires a verified primary email.",
        ));
    }

    sqlx::query(
        r#"
        INSERT INTO mfa_factors (
          id,
          principal_id,
          factor_type,
          status,
          label,
          factor_data,
          created_at,
          confirmed_at
        )
        SELECT $1, $2, 'email', 'active', $3, $4, NOW(), NOW()
        WHERE NOT EXISTS (
          SELECT 1
          FROM mfa_factors
          WHERE principal_id = $2
            AND factor_type = 'email'
            AND status = 'active'
        )
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(&email)
    .bind(serde_json::json!({
        "email": email,
        "system_default": true,
    }))
    .execute(db)
    .await?;

    Ok(())
}

pub async fn send_login_code(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    auth_state_id: Uuid,
    principal_id: Uuid,
) -> Result<(), AppError> {
    ensure_primary_email_factor(db, principal_id).await?;

    let recipients = verified_email_recipients(db, principal_id).await?;
    if recipients.is_empty() {
        return Err(AppError::forbidden(
            "email_mfa_requires_verified_email",
            "Email MFA requires at least one verified email.",
        ));
    }

    let code = generate_email_code();
    nvbes_redis::cache::cache_set_json(
        redis,
        &email_mfa_code_key(auth_state_id, principal_id),
        &CachedEmailMfaCode {
            principal_id,
            auth_state_id,
            code_hash: token_hash(&code),
            created_at: Utc::now(),
        },
        EMAIL_MFA_CODE_TTL_SECONDS,
    )
    .await
    .map_err(|err| AppError::internal("email_mfa_code_store_failed", err.to_string()))?;

    let code_hash = token_hash(&code);
    for email in recipients {
        crate::email::jobs::enqueue_email_job_tx(
            db,
            redis,
            crate::email::jobs::EmailSendPayload {
                to_email: email.clone(),
                to_name: None,
                subject: "Your nvbes sign-in code".to_string(),
                html_body: format!(
                    "<p>Your nvbes sign-in code is:</p><p><strong>{code}</strong></p><p>This code expires in 5 minutes.</p>"
                ),
                text_body: Some(format!(
                    "Your nvbes sign-in code is: {code}\n\nThis code expires in 5 minutes."
                )),
                business_type: "verification".to_string(),
            },
            &format!(
                "email-mfa:{}:{}:{}:{}",
                principal_id,
                auth_state_id,
                token_hash(&email),
                code_hash
            ),
        )
        .await?;
    }
    Ok(())
}

async fn verified_email_recipients(
    db: &PgPool,
    principal_id: Uuid,
) -> Result<Vec<String>, AppError> {
    let emails = sqlx::query_scalar::<_, String>(
        r#"
        SELECT email
        FROM user_email_addresses
        WHERE principal_id = $1
          AND deleted_at IS NULL
          AND verified_at IS NOT NULL
        ORDER BY is_primary DESC, created_at ASC
        "#,
    )
    .bind(principal_id)
    .fetch_all(db)
    .await?;
    Ok(emails)
}

pub async fn verify_login_code(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    auth_state_id: Uuid,
    principal_id: Uuid,
    code: &str,
) -> Result<(), AppError> {
    let key = email_mfa_code_key(auth_state_id, principal_id);
    let cached: Option<CachedEmailMfaCode> = nvbes_redis::cache::cache_get_json(redis, &key)
        .await
        .map_err(|err| AppError::internal("email_mfa_code_read_failed", err.to_string()))?;
    let cached = cached
        .ok_or_else(|| AppError::forbidden("invalid_email_mfa_code", "Invalid email MFA code."))?;
    if cached.principal_id != principal_id
        || cached.auth_state_id != auth_state_id
        || cached.code_hash != token_hash(code)
    {
        return Err(AppError::forbidden(
            "invalid_email_mfa_code",
            "Invalid email MFA code.",
        ));
    }
    nvbes_redis::cache::cache_del(redis, &key)
        .await
        .map_err(|err| AppError::internal("email_mfa_code_consume_failed", err.to_string()))?;
    sqlx::query(
        "UPDATE mfa_factors SET last_used_at = NOW() WHERE principal_id = $1 AND factor_type = 'email' AND status = 'active'",
    )
    .bind(principal_id)
    .execute(db)
    .await?;
    Ok(())
}

fn email_mfa_code_key(auth_state_id: Uuid, principal_id: Uuid) -> String {
    format!("nvbes:identity:email-mfa-code:{auth_state_id}:{principal_id}")
}

fn generate_email_code() -> String {
    let mut rng = rand::rng();
    format!("{:06}", rng.random_range(0..1_000_000))
}
