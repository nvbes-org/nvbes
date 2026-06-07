use chrono::{DateTime, Duration as ChronoDuration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::rules::{
    enterprise_recovery_approval_status_is_selectable, enterprise_recovery_first_approval_metadata,
    enterprise_recovery_requires_second_approval, enterprise_recovery_review_delay,
    enterprise_recovery_review_pending_metadata, enterprise_recovery_review_window_open,
};
use crate::domains::auth::password::{
    db, generate_random_token, log_dev_token, normalize_email, token_hash,
};
use crate::domains::auth::risk::{self, RiskDecision, RiskEventInput};
use crate::domains::auth::types::*;
use crate::http::error::AppError;

pub async fn approve_enterprise_recovery(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    approver_user_id: Uuid,
    email: &str,
    reset_ttl_minutes: i64,
    environment: &str,
) -> Result<ApproveRecoveryResult, AppError> {
    let normalized_email = normalize_email(email);
    let mut tx = db.begin().await?;

    let row = db::find_recovery_request_for_approval(&mut tx, &normalized_email)
        .await?
        .ok_or_else(|| {
            AppError::not_found("recovery_request_not_found", "No recovery request found.")
        })?;

    use sqlx::Row;
    let target_principal_id: Uuid = row.get("principal_id");
    let tenant_id: Uuid = row.get("tenant_id");
    let tenant_kind: String = row.get("tenant_kind");
    let request_id: Uuid = row.get("request_id");
    let available_at: DateTime<Utc> = row.get("available_at");
    let approved_by_principal_id: Option<Uuid> = row.get("approved_by_principal_id");
    let review_available_at: Option<DateTime<Utc>> = row.get("review_available_at");
    let secondary_approved_by_principal_id: Option<Uuid> =
        row.get("secondary_approved_by_principal_id");

    if tenant_kind != "enterprise" {
        return Err(AppError::bad_request(
            "recovery_not_enterprise",
            "Enterprise recovery approval is only available for enterprise tenants.",
        ));
    }

    let approver_is_admin = db::check_approver_is_admin(db, tenant_id, approver_user_id).await?;
    if !approver_is_admin {
        return Err(AppError::forbidden(
            "recovery_approval_denied",
            "Only workspace owners or admins can approve enterprise recovery.",
        ));
    }

    if available_at > Utc::now() {
        return Err(AppError::forbidden(
            "recovery_cooldown_active",
            "Recovery cooldown is still active.",
        ));
    }

    let status = row.get::<String, _>("status");
    if !enterprise_recovery_approval_status_is_selectable(&status) {
        return Err(AppError::bad_request(
            "recovery_invalid_state",
            "Enterprise recovery is not ready for approval.",
        ));
    }

    if enterprise_recovery_requires_second_approval(
        approved_by_principal_id,
        secondary_approved_by_principal_id,
    ) {
        let review_available_at = Utc::now() + enterprise_recovery_review_delay();
        db::update_recovery_first_approval(
            &mut tx,
            request_id,
            approver_user_id,
            review_available_at,
        )
        .await?;
        db::insert_audit_event(
            &mut tx,
            tenant_id,
            approver_user_id,
            "enterprise.recovery.first_approved",
            target_principal_id,
            enterprise_recovery_first_approval_metadata(
                request_id,
                available_at,
                review_available_at,
            ),
        )
        .await?;
        db::insert_audit_event(
            &mut tx,
            tenant_id,
            approver_user_id,
            "enterprise.recovery.review_pending",
            target_principal_id,
            enterprise_recovery_review_pending_metadata(request_id, review_available_at),
        )
        .await?;
        tx.commit().await?;

        let _ = risk::record_event(
            db,
            RiskEventInput {
                principal_id: target_principal_id,
                session_id: None,
                device_id: None,
                event_type: "enterprise_recovery_first_approved".to_string(),
                ip_address: None,
                user_agent: None,
                risk_score: 10.0,
                risk_factors: serde_json::json!({
                    "request_id": request_id,
                    "approver_user_id": approver_user_id,
                }),
                decision: RiskDecision::Allow,
                metadata: serde_json::json!({
                    "available_at": available_at,
                    "review_available_at": review_available_at,
                }),
            },
        )
        .await;

        return Ok(ApproveRecoveryResult {
            success: true,
            available_at: Some(review_available_at),
            requires_second_approval: true,
        });
    }

    if approved_by_principal_id == Some(approver_user_id)
        || secondary_approved_by_principal_id == Some(approver_user_id)
    {
        return Err(AppError::forbidden(
            "recovery_second_approval_denied",
            "A different owner or admin must perform the second recovery approval.",
        ));
    }

    if status != "first_approved" {
        return Err(AppError::bad_request(
            "recovery_invalid_state",
            "Enterprise recovery is not ready for final approval.",
        ));
    }

    let review_available_at = review_available_at.ok_or_else(|| {
        AppError::bad_request(
            "recovery_review_window_missing",
            "Enterprise recovery is missing the admin review window.",
        )
    })?;
    if !enterprise_recovery_review_window_open(review_available_at, Utc::now()) {
        return Err(AppError::forbidden(
            "recovery_review_window_active",
            "Enterprise recovery is still in the admin review window.",
        ));
    }

    let token = generate_random_token();
    log_dev_token(&token, environment, "enterprise_recovery_reset");
    let token_expires_at = Utc::now() + ChronoDuration::minutes(reset_ttl_minutes);
    let token_hash_value = token_hash(&token);

    db::update_recovery_final_approval(
        &mut tx,
        request_id,
        approver_user_id,
        &token_hash_value,
        token_expires_at,
    )
    .await?;
    db::reset::upsert_password_reset_token(
        &mut tx,
        redis,
        target_principal_id,
        &token_hash_value,
        token_expires_at,
    )
    .await?;

    tx.commit().await?;

    let _ = risk::record_event(
        db,
        RiskEventInput {
            principal_id: target_principal_id,
            session_id: None,
            device_id: None,
            event_type: "enterprise_recovery_approved".to_string(),
            ip_address: None,
            user_agent: None,
            risk_score: 10.0,
            risk_factors: serde_json::json!({
                "request_id": request_id,
                "first_approver_user_id": approved_by_principal_id,
                "second_approver_user_id": approver_user_id,
            }),
            decision: RiskDecision::Allow,
            metadata: serde_json::json!({
                "available_at": available_at,
                "review_available_at": review_available_at,
            }),
        },
    )
    .await;

    Ok(ApproveRecoveryResult {
        success: true,
        available_at: Some(available_at),
        requires_second_approval: false,
    })
}
