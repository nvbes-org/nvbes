use chrono::{DateTime, Duration, Utc};
use serde_json::Value;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::domains::auth::risk::RiskDecision;
use crate::http::error::AppError;

pub enum RecoveryReviewDisposition {
    Pending { request_id: Option<Uuid> },
    Approved { request_id: Uuid, tenant_id: Uuid },
}

pub async fn queue_or_authorize(
    db: &PgPool,
    principal_id: Uuid,
    email: &str,
    risk_score: f64,
    risk_factors: &Value,
) -> Result<RecoveryReviewDisposition, AppError> {
    let mut tx = db.begin().await?;
    let policy = sqlx::query(
        r#"
        SELECT p.tenant_id, t.recovery_review_min_age_hours
        FROM principals p
        INNER JOIN tenants t ON t.id = p.tenant_id
        WHERE p.id = $1
        "#,
    )
    .bind(principal_id)
    .fetch_one(&mut *tx)
    .await?;
    let tenant_id: Uuid = policy.get("tenant_id");
    let min_age_hours: i32 = policy.get("recovery_review_min_age_hours");
    sqlx::query("SELECT set_config('nvbes.tenant_id', $1, true)")
        .bind(tenant_id.to_string())
        .execute(&mut *tx)
        .await?;

    let active = sqlx::query(
        r#"
        SELECT id, status, review_available_at, approval_expires_at, reset_token_expires_at,
               updated_at
        FROM enterprise_password_recovery_requests
        WHERE principal_id = $1
          AND status IN ('pending', 'approved', 'issuing', 'token_issued')
        FOR UPDATE
        "#,
    )
    .bind(principal_id)
    .fetch_optional(&mut *tx)
    .await?;

    if let Some(active) = active {
        let request_id: Uuid = active.get("id");
        let status: String = active.get("status");
        let review_available_at: Option<DateTime<Utc>> = active.get("review_available_at");
        let approval_expires_at: Option<DateTime<Utc>> = active.get("approval_expires_at");
        let reset_token_expires_at: Option<DateTime<Utc>> = active.get("reset_token_expires_at");
        let updated_at: DateTime<Utc> = active.get("updated_at");
        if (matches!(status.as_str(), "approved" | "issuing")
            && approval_expires_at.is_none_or(|value| value <= Utc::now()))
            || (status == "token_issued"
                && reset_token_expires_at.is_none_or(|value| value <= Utc::now()))
        {
            sqlx::query(
                r#"
                UPDATE enterprise_password_recovery_requests
                SET status = 'expired', updated_at = NOW()
                WHERE id = $1 AND status IN ('approved', 'issuing', 'token_issued')
                "#,
            )
            .bind(request_id)
            .execute(&mut *tx)
            .await?;
        } else {
            let claimable_approval = status == "approved"
                || (status == "issuing" && updated_at <= Utc::now() - Duration::minutes(5));
            if claimable_approval
                && review_available_at.is_some_and(|value| value <= Utc::now())
                && approval_expires_at.is_some_and(|value| value > Utc::now())
            {
                sqlx::query(
                    r#"
                    UPDATE enterprise_password_recovery_requests
                    SET status = 'issuing', updated_at = NOW()
                    WHERE id = $1 AND status IN ('approved', 'issuing')
                    "#,
                )
                .bind(request_id)
                .execute(&mut *tx)
                .await?;
                tx.commit().await?;
                return Ok(RecoveryReviewDisposition::Approved {
                    request_id,
                    tenant_id,
                });
            }

            sqlx::query(
                r#"
                UPDATE enterprise_password_recovery_requests
                SET risk_score = GREATEST(risk_score, $2),
                    risk_factors = risk_factors || $3::jsonb,
                    updated_at = NOW()
                WHERE id = $1
                "#,
            )
            .bind(request_id)
            .bind(risk_score)
            .bind(risk_factors)
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            return Ok(RecoveryReviewDisposition::Pending { request_id: None });
        }
    }

    let rejection_cooldown_active = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
          SELECT 1
          FROM enterprise_password_recovery_requests
          WHERE principal_id = $1
            AND status = 'rejected'
            AND rejected_at > NOW() - make_interval(hours => $2::int)
        )
        "#,
    )
    .bind(principal_id)
    .bind(min_age_hours)
    .fetch_one(&mut *tx)
    .await?;
    if rejection_cooldown_active {
        tx.commit().await?;
        return Ok(RecoveryReviewDisposition::Pending { request_id: None });
    }

    let review_available_at = Utc::now() + Duration::hours(i64::from(min_age_hours));
    let request_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO enterprise_password_recovery_requests (
          principal_id, tenant_id, email, status, available_at, review_available_at,
          risk_score, risk_factors, created_at, updated_at
        )
        VALUES ($1, $2, $3, 'pending', $4, $4, $5, $6, NOW(), NOW())
        RETURNING id
        "#,
    )
    .bind(principal_id)
    .bind(tenant_id)
    .bind(email)
    .bind(review_available_at)
    .bind(risk_score)
    .bind(risk_factors)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(RecoveryReviewDisposition::Pending {
        request_id: Some(request_id),
    })
}

pub async fn mark_token_issued(
    db: &PgPool,
    tenant_id: Uuid,
    request_id: Uuid,
    token_hash: &str,
    expires_at: DateTime<Utc>,
) -> Result<(), AppError> {
    let mut tx = db.begin().await?;
    sqlx::query("SELECT set_config('nvbes.tenant_id', $1, true)")
        .bind(tenant_id.to_string())
        .execute(&mut *tx)
        .await?;
    let result = sqlx::query(
        r#"
        UPDATE enterprise_password_recovery_requests
        SET status = 'token_issued',
            reset_token_hash = $2,
            reset_token_expires_at = $3,
            updated_at = NOW()
        WHERE id = $1
          AND status = 'issuing'
          AND review_available_at <= NOW()
        "#,
    )
    .bind(request_id)
    .bind(token_hash)
    .bind(expires_at)
    .execute(&mut *tx)
    .await?;
    if result.rows_affected() != 1 {
        return Err(AppError::conflict(
            "recovery_review_state_changed",
            "The recovery review is no longer approved.",
        ));
    }
    tx.commit().await?;
    Ok(())
}

pub async fn rollback_token_delivery(
    db: &PgPool,
    tenant_id: Uuid,
    request_id: Uuid,
    token_hash: &str,
) -> Result<(), AppError> {
    let mut tx = db.begin().await?;
    sqlx::query("SELECT set_config('nvbes.tenant_id', $1, true)")
        .bind(tenant_id.to_string())
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        r#"
        UPDATE enterprise_password_recovery_requests
        SET status = CASE
                WHEN approval_expires_at > NOW() THEN 'approved'
                ELSE 'expired'
            END,
            reset_token_hash = NULL,
            reset_token_expires_at = NULL,
            updated_at = NOW()
        WHERE id = $1
          AND status = 'token_issued'
          AND reset_token_hash = $2
        "#,
    )
    .bind(request_id)
    .bind(token_hash)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn mark_consumed_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    principal_id: Uuid,
    token_hash: &str,
) -> Result<(), AppError> {
    let tenant_id = sqlx::query_scalar::<_, Uuid>("SELECT tenant_id FROM principals WHERE id = $1")
        .bind(principal_id)
        .fetch_one(&mut **tx)
        .await?;
    sqlx::query("SELECT set_config('nvbes.tenant_id', $1, true)")
        .bind(tenant_id.to_string())
        .execute(&mut **tx)
        .await?;
    sqlx::query(
        r#"
        UPDATE enterprise_password_recovery_requests
        SET status = 'consumed',
            consumed_at = NOW(),
            updated_at = NOW()
        WHERE principal_id = $1
          AND reset_token_hash = $2
          AND status = 'token_issued'
        "#,
    )
    .bind(principal_id)
    .bind(token_hash)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub fn requires_manual_review(decision: RiskDecision, locked: bool) -> bool {
    locked || matches!(decision, RiskDecision::Deny | RiskDecision::Lock)
}

#[cfg(test)]
mod tests {
    use super::requires_manual_review;
    use crate::domains::auth::risk::RiskDecision;

    #[test]
    fn only_high_risk_recovery_decisions_require_review() {
        assert!(!requires_manual_review(RiskDecision::Allow, false));
        assert!(!requires_manual_review(RiskDecision::StepUp, false));
        assert!(requires_manual_review(RiskDecision::Deny, false));
        assert!(requires_manual_review(RiskDecision::Lock, false));
        assert!(requires_manual_review(RiskDecision::Allow, true));
    }
}
