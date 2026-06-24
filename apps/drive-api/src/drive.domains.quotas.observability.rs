use super::logic::{
    STORAGE_CRITICAL_THRESHOLD_PERCENT, STORAGE_WARNING_THRESHOLD_PERCENT, crosses_threshold,
};
use super::types::StorageThresholdAuditInput;
use crate::http::error::AppError;

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

            crate::domains::audit::record_event_tx(
                tx,
                crate::domains::audit::AuditRecordInput {
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
