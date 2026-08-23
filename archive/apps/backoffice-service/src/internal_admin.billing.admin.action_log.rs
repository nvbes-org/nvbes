use sqlx::PgPool;
use uuid::Uuid;

use crate::billing_admin_types::BackofficeAccess;
use crate::billing_grpc_admin::BackofficeBillingAdminActionOutcome;
use crate::error::AppError;

pub(crate) fn audit_action(value: &str) -> Result<&'static str, AppError> {
    match value {
        "billing.credit_note.created" => Ok("billing.credit_note.created"),
        "billing.write_off.created" => Ok("billing.write_off.created"),
        "billing.refund_intent.created" => Ok("billing.refund_intent.created"),
        "billing.manual_comp.created" => Ok("billing.manual_comp.created"),
        "billing.provider_event.replayed" => Ok("billing.provider_event.replayed"),
        "billing.provider_migration.planned" => Ok("billing.provider_migration.planned"),
        "billing.grace_period.overridden" => Ok("billing.grace_period.overridden"),
        _ => Err(AppError::internal(
            "billing_grpc_invalid_audit_action",
            format!("unexpected billing admin audit action: {value}"),
        )),
    }
}

pub(crate) async fn record_admin_audit(
    db: &PgPool,
    access: BackofficeAccess,
    reason: &str,
    outcome: &BackofficeBillingAdminActionOutcome,
) -> Result<(), AppError> {
    let audit_action = audit_action(&outcome.audit_action)?;
    let mut tx = db.begin().await?;
    insert_admin_audit(
        &mut tx,
        access,
        audit_action,
        &outcome.target_type,
        outcome.object_id,
        reason,
    )
    .await?;
    tx.commit().await.map_err(AppError::from)
}

async fn insert_admin_audit(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    access: BackofficeAccess,
    action: &'static str,
    target_type: &str,
    target_id: Uuid,
    reason: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO audit_events (
           tenant_id, actor_principal_id, action, target_type, target_id, metadata, event_hash
         ) VALUES (
           $1, $2, $3, $4, $5,
           jsonb_build_object(
             'reason', $6,
             'object_links', jsonb_build_object(
               'target_id', $5::text,
               'target_type', $4
             ),
             'target_links', jsonb_build_object(
               'target_id', $5::text,
               'target_type', $4
             ),
             'changes', jsonb_build_array(jsonb_build_object(
               'field', 'billing_admin.action',
               'before', null,
               'after', 'recorded'
             ))
           ),
           gen_random_uuid()::text
         )",
    )
    .bind(access.tenant_id)
    .bind(access.actor_principal_id)
    .bind(action)
    .bind(target_type)
    .bind(target_id)
    .bind(reason)
    .execute(tx.as_mut())
    .await?;
    Ok(())
}
