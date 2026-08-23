use sqlx::{Postgres, Transaction};
use tonic::Status;
use uuid::Uuid;

use crate::grpc::{service_status::sql_status, user_access::AccessScope};

pub async fn insert_member_audit(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    scope: AccessScope,
    workspace_ids: &[Uuid],
    actor_id: Uuid,
    action: &'static str,
    target_principal_id: Uuid,
    metadata: serde_json::Value,
) -> Result<(), Status> {
    if matches!(scope, AccessScope::Tenant) {
        insert_audit(
            tx,
            tenant_id,
            None,
            actor_id,
            action,
            target_principal_id,
            metadata,
        )
        .await?;
        return Ok(());
    }
    for workspace_id in workspace_ids {
        insert_audit(
            tx,
            tenant_id,
            Some(*workspace_id),
            actor_id,
            action,
            target_principal_id,
            metadata.clone(),
        )
        .await?;
    }
    Ok(())
}

pub fn merge_workspace_ids(left: &[Uuid], right: &[Uuid]) -> Vec<Uuid> {
    let mut merged = left.to_vec();
    for id in right {
        if !merged.contains(id) {
            merged.push(*id);
        }
    }
    merged
}

async fn insert_audit(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    workspace_id: Option<Uuid>,
    actor_id: Uuid,
    action: &'static str,
    target_principal_id: Uuid,
    metadata: serde_json::Value,
) -> Result<(), Status> {
    sqlx::query(
        r#"
        INSERT INTO audit_events (
          tenant_id, workspace_id, actor_principal_id, action, target_type, target_id,
          metadata, event_hash, created_at
        )
        VALUES ($1, $2, $3, $4, 'principal', $5, $6, gen_random_uuid()::text, NOW())
        "#,
    )
    .bind(tenant_id)
    .bind(workspace_id)
    .bind(actor_id)
    .bind(action)
    .bind(target_principal_id)
    .bind(metadata)
    .execute(&mut **tx)
    .await
    .map_err(sql_status)?;
    Ok(())
}
