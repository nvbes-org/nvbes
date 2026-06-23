use super::logic::{
    STORAGE_CRITICAL_THRESHOLD_PERCENT, STORAGE_WARNING_THRESHOLD_PERCENT, crosses_threshold,
};
use super::types::{AuditEventInput, StorageThresholdAuditInput};
use crate::http::error::AppError;

pub async fn insert_audit_event(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    input: AuditEventInput<'_>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO audit_events (
          workspace_id,
          actor_user_id,
          actor_principal_id,
          action,
          target_type,
          target_id,
          ip,
          user_agent,
          metadata
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7::inet, $8, $9)
        "#,
    )
    .bind(input.workspace_id)
    .bind(input.actor_user_id)
    .bind(input.actor_principal_id)
    .bind(input.action)
    .bind(input.target_type)
    .bind(input.target_id)
    .bind(input.ip)
    .bind(input.user_agent)
    .bind(sqlx::types::Json(input.metadata))
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub async fn insert_storage_threshold_audits(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    input: StorageThresholdAuditInput<'_>,
) -> Result<(), AppError> {
    for threshold in [
        STORAGE_WARNING_THRESHOLD_PERCENT,
        STORAGE_CRITICAL_THRESHOLD_PERCENT,
    ] {
        if crosses_threshold(
            input.before.used_storage_bytes,
            input.after_used_storage_bytes,
            input.before.included_storage_bytes,
            threshold,
        ) {
            let action = if threshold == STORAGE_CRITICAL_THRESHOLD_PERCENT {
                "quota.storage_critical"
            } else {
                "quota.storage_warning"
            };

            insert_audit_event(
                tx,
                AuditEventInput {
                    workspace_id: input.workspace_id,
                    actor_user_id: input.actor_user_id,
                    actor_principal_id: Some(input.actor_principal_id),
                    action,
                    target_type: "storage_object",
                    target_id: Some(input.storage_object_id),
                    ip: input.ip,
                    user_agent: input.user_agent,
                    metadata: serde_json::json!({
                        "threshold_percent": threshold,
                        "used_storage_bytes": input.after_used_storage_bytes,
                        "included_storage_bytes": input.before.included_storage_bytes,
                    }),
                },
            )
            .await?;
        }
    }

    Ok(())
}
