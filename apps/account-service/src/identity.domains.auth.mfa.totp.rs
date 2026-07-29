use chrono::Utc;
use nvbes_core::config::AppConfig;
use nvbes_core::mfa::{
    TOTP_DIGITS, TOTP_PERIOD_SECONDS, TOTP_WINDOW, generate_totp_secret, provisioning_uri,
};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::domains::auth::types::*;
use crate::http::error::AppError;

#[path = "identity.domains.auth.mfa.totp.secret.rs"]
mod secret_cipher;

pub async fn begin_totp(
    db: &PgPool,
    config: &AppConfig,
    user_id: Uuid,
    input: TotpSetupInput,
) -> Result<TotpSetupResult, AppError> {
    let secret = generate_totp_secret();
    let factor_id = Uuid::new_v4();
    let now = Utc::now();
    let label = input
        .label
        .unwrap_or_else(|| "Authenticator app".to_string());
    let account_email: String =
        sqlx::query_scalar("SELECT email FROM users WHERE principal_id = $1")
            .bind(user_id)
            .fetch_one(db)
            .await?;
    let tenant_id: Uuid = sqlx::query_scalar("SELECT tenant_id FROM principals WHERE id = $1")
        .bind(user_id)
        .fetch_one(db)
        .await?;
    let secret_envelope = secret_cipher::encrypt(config, tenant_id, user_id, factor_id, &secret)?;

    sqlx::query(
        r#"
        INSERT INTO mfa_factors (
          id, principal_id, factor_type, status, label, totp_secret_envelope,
          totp_digits, totp_period_seconds, created_at
        )
        VALUES ($1, $2, 'totp', 'pending', $3, $4, $5, $6, $7)
        "#,
    )
    .bind(factor_id)
    .bind(user_id)
    .bind(&label)
    .bind(secret_envelope)
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
            assurance: None,
            phishing_resistant: false,
            backup_eligible: None,
            backup_state: None,
            sign_count: None,
            attestation_format: None,
            status: "pending".to_string(),
            label: Some(label.clone()),
            created_at: now,
            confirmed_at: None,
            last_used_at: None,
        },
        secret_base32: secret.clone(),
        provisioning_uri: provisioning_uri("Nvbes", &account_email, &secret),
    })
}

pub async fn confirm_totp(
    db: &PgPool,
    config: &AppConfig,
    user_id: Uuid,
    input: TotpConfirmInput,
) -> Result<TotpConfirmResult, AppError> {
    let row = sqlx::query(
        r#"
        SELECT mf.totp_secret_envelope, mf.totp_secret_base32, mf.label, mf.created_at,
               mf.status::text AS status, p.tenant_id
        FROM mfa_factors mf
        INNER JOIN principals p ON p.id = mf.principal_id
        WHERE mf.id = $1 AND mf.principal_id = $2 AND mf.factor_type = 'totp'
        LIMIT 1
        "#,
    )
    .bind(input.factor_id)
    .bind(user_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::not_found("factor_not_found", "Factor not found."))?;

    let tenant_id: Uuid = row.get("tenant_id");
    let (secret, migrated_envelope) = decrypt_or_migrate(
        config,
        tenant_id,
        user_id,
        input.factor_id,
        row.get("totp_secret_envelope"),
        row.get("totp_secret_base32"),
    )?;
    let counter = nvbes_core::mfa::verify_totp_code(&secret, &input.code, Utc::now(), TOTP_WINDOW)
        .ok_or_else(|| AppError::forbidden("invalid_totp_code", "Invalid TOTP code."))?;

    sqlx::query(
        r#"
        UPDATE mfa_factors
        SET status = 'active',
            confirmed_at = COALESCE(confirmed_at, NOW()),
            totp_last_used_counter = $3,
            last_used_at = NOW(),
            totp_secret_envelope = COALESCE($4, totp_secret_envelope),
            totp_secret_base32 = CASE WHEN $4::jsonb IS NULL THEN totp_secret_base32 ELSE NULL END
        WHERE id = $1 AND principal_id = $2
        "#,
    )
    .bind(input.factor_id)
    .bind(user_id)
    .bind(counter as i64)
    .bind(migrated_envelope)
    .execute(db)
    .await?;

    Ok(TotpConfirmResult {
        factor: MfaFactorView {
            id: input.factor_id,
            factor_type: "totp".to_string(),
            kind: None,
            assurance: None,
            phishing_resistant: false,
            backup_eligible: None,
            backup_state: None,
            sign_count: None,
            attestation_format: None,
            status: "active".to_string(),
            label: row.get("label"),
            created_at: row.get("created_at"),
            confirmed_at: Some(Utc::now()),
            last_used_at: Some(Utc::now()),
        },
        mfa_enabled: true,
    })
}

pub async fn verify_totp(
    db: &PgPool,
    config: &AppConfig,
    user_id: Uuid,
    code: &str,
) -> Result<(), AppError> {
    let rows = sqlx::query(
        r#"
        SELECT mf.id, mf.totp_secret_envelope, mf.totp_secret_base32,
               mf.totp_last_used_counter, p.tenant_id
        FROM mfa_factors mf
        INNER JOIN principals p ON p.id = mf.principal_id
        WHERE mf.principal_id = $1 AND mf.factor_type = 'totp' AND mf.status = 'active'
        "#,
    )
    .bind(user_id)
    .fetch_all(db)
    .await?;

    for row in rows {
        let factor_id: Uuid = row.get("id");
        let tenant_id: Uuid = row.get("tenant_id");
        let Ok((secret, migrated_envelope)) = decrypt_or_migrate(
            config,
            tenant_id,
            user_id,
            factor_id,
            row.get("totp_secret_envelope"),
            row.get("totp_secret_base32"),
        ) else {
            tracing::error!(%factor_id, "TOTP factor secret could not be decrypted");
            continue;
        };
        let last_counter: i64 = row.get("totp_last_used_counter");
        if let Some(counter) =
            nvbes_core::mfa::verify_totp_code(&secret, code, Utc::now(), TOTP_WINDOW)
            && counter as i64 > last_counter
        {
            let updated = sqlx::query(
                r#"
                UPDATE mfa_factors
                SET totp_last_used_counter = $2,
                    last_used_at = NOW(),
                    totp_secret_envelope = COALESCE($3, totp_secret_envelope),
                    totp_secret_base32 = CASE
                        WHEN $3::jsonb IS NULL THEN totp_secret_base32
                        ELSE NULL
                    END
                WHERE id = $1 AND totp_last_used_counter < $2
                "#,
            )
            .bind(factor_id)
            .bind(counter as i64)
            .bind(migrated_envelope)
            .execute(db)
            .await?;
            if updated.rows_affected() == 1 {
                return Ok(());
            }
        }
    }

    Err(AppError::forbidden(
        "invalid_totp_code",
        "Invalid TOTP code.",
    ))
}

fn decrypt_or_migrate(
    config: &AppConfig,
    tenant_id: Uuid,
    principal_id: Uuid,
    factor_id: Uuid,
    envelope: Option<serde_json::Value>,
    legacy_secret: Option<String>,
) -> Result<(String, Option<serde_json::Value>), AppError> {
    if let Some(envelope) = envelope {
        return secret_cipher::decrypt(config, tenant_id, principal_id, factor_id, envelope)
            .map(|secret| (secret, None));
    }

    let legacy_secret = legacy_secret
        .ok_or_else(|| AppError::internal("totp_secret_missing", "TOTP secret is missing."))?;
    let envelope =
        secret_cipher::encrypt(config, tenant_id, principal_id, factor_id, &legacy_secret)?;
    Ok((legacy_secret, Some(envelope)))
}
