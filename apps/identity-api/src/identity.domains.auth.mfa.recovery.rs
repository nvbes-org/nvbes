use serde_json::json;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::domains::auth::risk::{self, RiskDecision, RiskEventInput};
use crate::domains::auth::{db, password, types::*};
use crate::http::error::AppError;
use nvbes_core::mfa::random_recovery_code;
use password::token_hash;

pub async fn has_active_recovery_codes(db: &PgPool, user_id: Uuid) -> Result<bool, AppError> {
    let exists = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
          SELECT 1
          FROM mfa_factors
          WHERE principal_id = $1
            AND factor_type = 'recovery_code'
            AND status = 'active'
        )
        "#,
    )
    .bind(user_id)
    .fetch_one(db)
    .await?;
    Ok(exists)
}

pub async fn verify_recovery(db: &PgPool, user_id: Uuid, code: &str) -> Result<(), AppError> {
    let row = sqlx::query(
        r#"
        SELECT id, factor_data
        FROM mfa_factors
        WHERE principal_id = $1 AND factor_type = 'recovery_code' AND status = 'active'
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::forbidden("no_recovery_codes", "No recovery codes configured."))?;

    let factor_id: Uuid = row.get("id");
    let data: serde_json::Value = row.get("factor_data");
    let codes = data["codes"]
        .as_array()
        .ok_or_else(|| AppError::internal("invalid_factor_data", "Invalid recovery codes data."))?;

    let code_hash = token_hash(code);
    let mut new_codes = Vec::new();
    let mut found = false;

    for c in codes {
        if c.as_str() == Some(&code_hash) && !found {
            found = true;
        } else {
            new_codes.push(c.clone());
        }
    }

    if !found {
        return Err(AppError::forbidden(
            "invalid_recovery_code",
            "Invalid recovery code.",
        ));
    }

    sqlx::query("UPDATE mfa_factors SET factor_data = $2, last_used_at = NOW() WHERE id = $1")
        .bind(factor_id)
        .bind(json!({ "codes": new_codes }))
        .execute(db)
        .await?;

    Ok(())
}

pub async fn generate_recovery(
    db: &PgPool,
    user_id: Uuid,
    password: &str,
) -> Result<RecoveryCodesResult, AppError> {
    let (risk_score, decision, risk_factors) = risk::current_state_summary(db, user_id).await?;
    if matches!(decision, RiskDecision::Deny | RiskDecision::Lock)
        || risk::should_lock_password_reset(db, user_id).await?
    {
        let _ = risk::record_event(
            db,
            RiskEventInput {
                principal_id: user_id,
                session_id: None,
                device_id: None,
                event_type: "recovery_generation_blocked".to_string(),
                ip_address: None,
                user_agent: None,
                risk_score,
                risk_factors,
                decision: RiskDecision::Deny,
                metadata: json!({}),
            },
        )
        .await;

        return Err(AppError::forbidden(
            "risk_policy_blocked",
            "Recovery codes are temporarily blocked for this account.",
        ));
    }

    let user = db::fetch_user_record(db, user_id).await?;
    password::verify_password(
        user.password_hash.as_deref().ok_or_else(|| {
            AppError::forbidden(
                "password_missing",
                "No password is configured for this account.",
            )
        })?,
        password,
    )?;

    let mut codes = Vec::with_capacity(10);
    for _ in 0..10 {
        let code = random_recovery_code();
        codes.push(code);
    }

    let factor_id = Uuid::new_v4();
    sqlx::query(
        r#"
        UPDATE mfa_factors
        SET status = 'revoked',
            last_used_at = NOW()
        WHERE principal_id = $1
          AND factor_type = 'recovery_code'
          AND status = 'active'
        "#,
    )
    .bind(user_id)
    .execute(db)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO mfa_factors (id, principal_id, factor_type, status, label, factor_data, confirmed_at, created_at)
        VALUES ($1, $2, 'recovery_code', 'active', 'Recovery codes', $3, NOW(), NOW())
        "#,
    )
    .bind(factor_id)
    .bind(user_id)
    .bind(json!({
        "codes": codes.iter().map(|code| token_hash(code)).collect::<Vec<_>>()
    }))
    .execute(db)
    .await?;

    let _ = risk::record_event(
        db,
        RiskEventInput {
            principal_id: user_id,
            session_id: None,
            device_id: None,
            event_type: "recovery_codes_generated".to_string(),
            ip_address: None,
            user_agent: None,
            risk_score: 10.0,
            risk_factors: json!({
                "count": codes.len(),
            }),
            decision: RiskDecision::Allow,
            metadata: json!({}),
        },
    )
    .await;

    Ok(RecoveryCodesResult { codes })
}
