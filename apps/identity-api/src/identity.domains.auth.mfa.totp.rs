use chrono::Utc;
use nvbes_core::mfa::{
    TOTP_DIGITS, TOTP_PERIOD_SECONDS, TOTP_WINDOW, generate_totp_secret, provisioning_uri,
};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::domains::auth::types::*;
use crate::http::error::AppError;

pub async fn begin_totp(
    db: &PgPool,
    user_id: Uuid,
    rp_id: &str,
    input: TotpSetupInput,
) -> Result<TotpSetupResult, AppError> {
    let secret = generate_totp_secret();
    let factor_id = Uuid::new_v4();
    let now = Utc::now();
    let label = input
        .label
        .unwrap_or_else(|| "Authenticator app".to_string());

    sqlx::query(
        r#"
        INSERT INTO mfa_factors (
          id, principal_id, factor_type, status, label, totp_secret_base32, totp_digits, totp_period_seconds, created_at
        )
        VALUES ($1, $2, 'totp', 'pending', $3, $4, $5, $6, $7)
        "#,
    )
    .bind(factor_id)
    .bind(user_id)
    .bind(&label)
    .bind(&secret)
    .bind(TOTP_DIGITS as i16)
    .bind(TOTP_PERIOD_SECONDS as i32)
    .bind(now)
    .execute(db)
    .await?;

    Ok(TotpSetupResult {
        factor: MfaFactorView {
            id: factor_id,
            factor_type: "totp".to_string(),
            kind: None,
            status: "pending".to_string(),
            label: Some(label.clone()),
            created_at: now,
            confirmed_at: None,
            last_used_at: None,
        },
        secret_base32: secret.clone(),
        provisioning_uri: provisioning_uri(rp_id, &label, &secret),
    })
}

pub async fn confirm_totp(
    db: &PgPool,
    user_id: Uuid,
    input: TotpConfirmInput,
) -> Result<TotpConfirmResult, AppError> {
    let row = sqlx::query(
        r#"
        SELECT totp_secret_base32, label, created_at, status::text AS status
        FROM mfa_factors
        WHERE id = $1 AND principal_id = $2 AND factor_type = 'totp'
        LIMIT 1
        "#,
    )
    .bind(input.factor_id)
    .bind(user_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::not_found("factor_not_found", "Factor not found."))?;

    let secret: Option<String> = row.get("totp_secret_base32");
    let secret = secret
        .ok_or_else(|| AppError::bad_request("validation_failed", "TOTP secret is missing."))?;
    let counter = nvbes_core::mfa::verify_totp_code(&secret, &input.code, Utc::now(), TOTP_WINDOW)
        .ok_or_else(|| AppError::forbidden("invalid_totp_code", "Invalid TOTP code."))?;

    sqlx::query(
        r#"
        UPDATE mfa_factors
        SET status = 'active', confirmed_at = COALESCE(confirmed_at, NOW()), totp_last_used_counter = $3, last_used_at = NOW()
        WHERE id = $1 AND principal_id = $2
        "#,
    )
    .bind(input.factor_id)
    .bind(user_id)
    .bind(counter as i64)
    .execute(db)
    .await?;

    Ok(TotpConfirmResult {
        factor: MfaFactorView {
            id: input.factor_id,
            factor_type: "totp".to_string(),
            kind: None,
            status: "active".to_string(),
            label: row.get("label"),
            created_at: row.get("created_at"),
            confirmed_at: Some(Utc::now()),
            last_used_at: Some(Utc::now()),
        },
        mfa_enabled: true,
    })
}

pub async fn verify_totp(db: &PgPool, user_id: Uuid, code: &str) -> Result<(), AppError> {
    let rows = sqlx::query(
        r#"
        SELECT id, totp_secret_base32, totp_last_used_counter
        FROM mfa_factors
        WHERE principal_id = $1 AND factor_type = 'totp' AND status = 'active'
        "#,
    )
    .bind(user_id)
    .fetch_all(db)
    .await?;

    for row in rows {
        let secret: String = row.get("totp_secret_base32");
        let last_counter: i64 = row.get("totp_last_used_counter");
        if let Some(counter) =
            nvbes_core::mfa::verify_totp_code(&secret, code, Utc::now(), TOTP_WINDOW)
            && counter as i64 > last_counter
        {
            sqlx::query("UPDATE mfa_factors SET totp_last_used_counter = $2, last_used_at = NOW() WHERE id = $1")
                    .bind(row.get::<Uuid, _>("id"))
                    .bind(counter as i64)
                    .execute(db)
                    .await?;
            return Ok(());
        }
    }

    Err(AppError::forbidden(
        "invalid_totp_code",
        "Invalid TOTP code.",
    ))
}
