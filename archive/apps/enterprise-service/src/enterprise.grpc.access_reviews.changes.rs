use sqlx::{Postgres, Transaction};
use tonic::Status;
use uuid::Uuid;

use super::AccessReviewItemRow;
use crate::grpc::service_status::{parse_uuid, sql_status};

pub async fn apply_change(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    item: &AccessReviewItemRow,
    target_role: &str,
) -> Result<(), Status> {
    validate_workspace_role(target_role)?;
    match item.item_type.as_str() {
        "role" => change_role(tx, tenant_id, item, target_role).await,
        "service_account" => change_service_account_role(tx, tenant_id, item, target_role).await,
        _ => Err(Status::invalid_argument(
            "only role and service account review items can apply role changes",
        )),
    }
}

async fn change_role(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    item: &AccessReviewItemRow,
    target_role: &str,
) -> Result<(), Status> {
    let principal_id = parse_role_subject(&item.subject_id)?;
    let workspace_id = item
        .workspace_id
        .ok_or_else(|| Status::invalid_argument("role review item is missing workspace scope"))?;
    change_workspace_role(tx, tenant_id, workspace_id, principal_id, target_role).await
}

async fn change_service_account_role(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    item: &AccessReviewItemRow,
    target_role: &str,
) -> Result<(), Status> {
    let principal_id = parse_uuid(&item.subject_id, "subject_principal_id")?;
    let workspace_id = item.workspace_id.ok_or_else(|| {
        Status::invalid_argument("service account review item is missing workspace scope")
    })?;
    change_workspace_role(tx, tenant_id, workspace_id, principal_id, target_role).await
}

async fn change_workspace_role(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    workspace_id: Uuid,
    principal_id: Uuid,
    target_role: &str,
) -> Result<(), Status> {
    ensure_not_last_workspace_owner_after_change(
        tx,
        tenant_id,
        workspace_id,
        principal_id,
        target_role,
    )
    .await?;
    let rows = sqlx::query(
        r#"
        UPDATE workspace_memberships wm
        SET role = $4::workspace_member_role, status = 'active', source = 'manual', updated_at = NOW()
        FROM workspaces w
        WHERE w.id = wm.workspace_id
          AND w.tenant_id = $1
          AND wm.workspace_id = $2
          AND wm.principal_id = $3
        "#,
    )
    .bind(tenant_id)
    .bind(workspace_id)
    .bind(principal_id)
    .bind(target_role)
    .execute(&mut **tx)
    .await
    .map_err(sql_status)?
    .rows_affected();
    if rows == 0 {
        return Err(Status::not_found(
            "enterprise access review workspace membership was not found",
        ));
    }
    Ok(())
}

async fn ensure_not_last_workspace_owner_after_change(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    workspace_id: Uuid,
    principal_id: Uuid,
    target_role: &str,
) -> Result<(), Status> {
    if target_role == "owner" {
        return Ok(());
    }
    let ownerless_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM workspace_memberships current_owner
        INNER JOIN workspaces w ON w.id = current_owner.workspace_id
        WHERE w.tenant_id = $1
          AND current_owner.workspace_id = $2
          AND current_owner.principal_id = $3
          AND current_owner.role = 'owner'
          AND current_owner.status = 'active'
          AND NOT EXISTS (
            SELECT 1
            FROM workspace_memberships other_owner
            WHERE other_owner.workspace_id = current_owner.workspace_id
              AND other_owner.principal_id <> $3
              AND other_owner.role = 'owner'
              AND other_owner.status = 'active'
          )
        "#,
    )
    .bind(tenant_id)
    .bind(workspace_id)
    .bind(principal_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(sql_status)?;
    if ownerless_count > 0 {
        return Err(Status::failed_precondition(
            "every workspace must retain at least one active owner",
        ));
    }
    Ok(())
}

fn validate_workspace_role(role: &str) -> Result<(), Status> {
    match role.trim() {
        "owner" | "admin" | "member" | "viewer" => Ok(()),
        _ => Err(Status::invalid_argument(
            "target_role must be owner, admin, member, or viewer",
        )),
    }
}

fn parse_role_subject(value: &str) -> Result<Uuid, Status> {
    let principal_id = value
        .split_once(':')
        .map_or(value, |(principal_id, _)| principal_id);
    parse_uuid(principal_id, "subject_principal_id")
}
