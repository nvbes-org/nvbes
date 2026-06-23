use serde::Serialize;
use serde_json::{Value, json};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::billing_admin_types::BackofficeAccess;
use crate::error::AppError;

#[derive(Debug, Serialize)]
pub(crate) struct EntitlementActionResult {
    pub(crate) object_id: Uuid,
    pub(crate) action_kind: &'static str,
    pub(crate) status: &'static str,
    pub(crate) published_change_count: i64,
    pub(crate) audit_action: &'static str,
}

pub(crate) struct EntitlementActionInput {
    pub(crate) action_kind: &'static str,
    pub(crate) feature_code: Option<String>,
    pub(crate) quota_code: Option<String>,
    pub(crate) quantity: Option<i64>,
    pub(crate) reason: String,
    pub(crate) metadata: Value,
    pub(crate) audit_action: &'static str,
    pub(crate) status: &'static str,
    pub(crate) published_change_count: i64,
}

pub(crate) async fn insert_entitlement_action(
    db: &PgPool,
    access: BackofficeAccess,
    workspace_id: Uuid,
    input: EntitlementActionInput,
) -> Result<EntitlementActionResult, AppError> {
    validate_reason(&input.reason)?;
    let mut tx = db.begin().await?;
    let action_id = insert_action_row(&mut tx, access, workspace_id, &input).await?;
    insert_entitlement_audit(
        &mut tx,
        access,
        input.audit_action,
        action_id,
        &input.reason,
    )
    .await?;
    tx.commit().await?;
    Ok(action_result(action_id, input))
}

pub(crate) async fn publish_changes(
    db: &PgPool,
    access: BackofficeAccess,
    workspace_id: Uuid,
    reason: String,
) -> Result<EntitlementActionResult, AppError> {
    validate_reason(&reason)?;
    let mut tx = db.begin().await?;
    let published_change_count = sqlx::query(
        "UPDATE billing_entitlement_changes
         SET published_at = NOW()
         WHERE tenant_id = $1 AND published_at IS NULL",
    )
    .bind(access.tenant_id)
    .execute(tx.as_mut())
    .await?
    .rows_affected() as i64;
    let input = EntitlementActionInput {
        action_kind: "publish_changes",
        feature_code: None,
        quota_code: None,
        quantity: None,
        reason,
        metadata: json!({ "published_change_count": published_change_count }),
        audit_action: "entitlements.changes.published",
        status: "published",
        published_change_count,
    };
    let action_id = insert_action_row(&mut tx, access, workspace_id, &input).await?;
    insert_entitlement_audit(
        &mut tx,
        access,
        input.audit_action,
        action_id,
        &input.reason,
    )
    .await?;
    tx.commit().await?;
    Ok(action_result(action_id, input))
}

async fn insert_action_row(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    access: BackofficeAccess,
    workspace_id: Uuid,
    input: &EntitlementActionInput,
) -> Result<Uuid, AppError> {
    let row = sqlx::query(
        "INSERT INTO internal_admin_entitlement_actions (
           tenant_id, workspace_id, actor_principal_id, action_kind, feature_code,
           quota_code, quantity, status, reason, metadata, published_at
         ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
           CASE WHEN $8 = 'published' THEN NOW() ELSE NULL END
         ) RETURNING id",
    )
    .bind(access.tenant_id)
    .bind(workspace_id)
    .bind(access.actor_principal_id)
    .bind(input.action_kind)
    .bind(&input.feature_code)
    .bind(&input.quota_code)
    .bind(input.quantity)
    .bind(input.status)
    .bind(&input.reason)
    .bind(&input.metadata)
    .fetch_one(tx.as_mut())
    .await?;
    Ok(row.get("id"))
}

async fn insert_entitlement_audit(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    access: BackofficeAccess,
    action: &'static str,
    target_id: Uuid,
    reason: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO audit_events (
           tenant_id, actor_principal_id, action, target_type, target_id, metadata, event_hash
         ) VALUES (
           $1, $2, $3, 'internal_admin_entitlement_action', $4,
           jsonb_build_object('reason', $5), gen_random_uuid()::text
         )",
    )
    .bind(access.tenant_id)
    .bind(access.actor_principal_id)
    .bind(action)
    .bind(target_id)
    .bind(reason)
    .execute(tx.as_mut())
    .await?;
    Ok(())
}

fn action_result(action_id: Uuid, input: EntitlementActionInput) -> EntitlementActionResult {
    EntitlementActionResult {
        object_id: action_id,
        action_kind: input.action_kind,
        status: input.status,
        published_change_count: input.published_change_count,
        audit_action: input.audit_action,
    }
}

pub(crate) fn validate_code(value: &str, field: &str) -> Result<(), AppError> {
    let trimmed = value.trim();
    if trimmed.len() >= 2 && trimmed.len() <= 96 {
        return Ok(());
    }
    Err(AppError::bad_request(
        "invalid_entitlement_code",
        format!("{field} must contain between 2 and 96 characters."),
    ))
}

pub(crate) fn validate_reason(value: &str) -> Result<(), AppError> {
    let len = value.trim().len();
    if (8..=500).contains(&len) {
        return Ok(());
    }
    Err(AppError::bad_request(
        "invalid_entitlement_reason",
        "Entitlement action reason must contain between 8 and 500 characters.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entitlement_codes_must_be_bounded() {
        assert!(validate_code("seats", "quota_code").is_ok());
        assert!(validate_code("x", "quota_code").is_err());
    }

    #[test]
    fn entitlement_reasons_must_be_operationally_useful() {
        assert!(validate_reason("ticket ENT-123 approved").is_ok());
        assert!(validate_reason("short").is_err());
    }
}
