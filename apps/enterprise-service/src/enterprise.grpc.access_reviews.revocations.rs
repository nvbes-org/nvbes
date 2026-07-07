use sqlx::{Postgres, Transaction};
use tonic::Status;
use uuid::Uuid;

use super::AccessReviewItemRow;
use crate::grpc::service_status::{parse_uuid, sql_status};

pub struct AppliedRevocation {
    pub target_id: Option<Uuid>,
}

pub async fn apply_revocation(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    item: &AccessReviewItemRow,
) -> Result<AppliedRevocation, Status> {
    match item.item_type.as_str() {
        "member" => revoke_member(tx, tenant_id, item).await,
        "role" => revoke_role(tx, tenant_id, item).await,
        "service_account" => revoke_service_account(tx, tenant_id, item).await,
        "oauth_client" => revoke_oauth_client(tx, tenant_id, item).await,
        _ => Err(Status::invalid_argument(
            "access review item type does not support revocation",
        )),
    }
}

async fn revoke_member(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    item: &AccessReviewItemRow,
) -> Result<AppliedRevocation, Status> {
    let principal_id = parse_uuid(&item.subject_id, "subject_principal_id")?;
    ensure_not_last_owner(tx, tenant_id, principal_id).await?;
    sqlx::query(
        r#"
        UPDATE tenant_memberships
        SET status = 'removed', updated_at = NOW()
        WHERE tenant_id = $1 AND principal_id = $2
        "#,
    )
    .bind(tenant_id)
    .bind(principal_id)
    .execute(&mut **tx)
    .await
    .map_err(sql_status)?;
    Ok(applied(Some(principal_id)))
}

async fn revoke_role(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    item: &AccessReviewItemRow,
) -> Result<AppliedRevocation, Status> {
    let principal_id = parse_role_subject(&item.subject_id)?;
    let workspace_id = item
        .workspace_id
        .ok_or_else(|| Status::invalid_argument("role review item is missing workspace scope"))?;
    ensure_not_last_workspace_owner(tx, tenant_id, principal_id, workspace_id).await?;
    sqlx::query(
        r#"
        UPDATE workspace_memberships
        SET status = 'removed', updated_at = NOW()
        WHERE workspace_id = $1 AND principal_id = $2
        "#,
    )
    .bind(workspace_id)
    .bind(principal_id)
    .execute(&mut **tx)
    .await
    .map_err(sql_status)?;
    Ok(applied(Some(principal_id)))
}

async fn revoke_service_account(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    item: &AccessReviewItemRow,
) -> Result<AppliedRevocation, Status> {
    let principal_id = parse_uuid(&item.subject_id, "subject_principal_id")?;
    if let Some(workspace_id) = item.workspace_id {
        ensure_not_last_workspace_owner(tx, tenant_id, principal_id, workspace_id).await?;
    }
    sqlx::query(
        "UPDATE principals SET status = 'revoked', updated_at = NOW() WHERE id = $1 AND tenant_id = $2",
    )
    .bind(principal_id)
    .bind(tenant_id)
    .execute(&mut **tx)
    .await
    .map_err(sql_status)?;
    sqlx::query(
        r#"
        UPDATE workspace_memberships wm
        SET status = 'removed', updated_at = NOW()
        FROM workspaces w
        WHERE w.id = wm.workspace_id AND w.tenant_id = $1 AND wm.principal_id = $2
        "#,
    )
    .bind(tenant_id)
    .bind(principal_id)
    .execute(&mut **tx)
    .await
    .map_err(sql_status)?;
    Ok(applied(Some(principal_id)))
}

async fn revoke_oauth_client(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    item: &AccessReviewItemRow,
) -> Result<AppliedRevocation, Status> {
    let client_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        UPDATE oauth_clients
        SET revoked_at = NOW(), updated_at = NOW()
        WHERE tenant_id = $1 AND client_id = $2 AND revoked_at IS NULL
        RETURNING id
        "#,
    )
    .bind(tenant_id)
    .bind(&item.subject_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(sql_status)?
    .ok_or_else(|| {
        Status::not_found("enterprise access review OAuth client target was not found")
    })?;
    Ok(applied(Some(client_id)))
}

fn parse_role_subject(value: &str) -> Result<Uuid, Status> {
    let principal_id = value
        .split_once(':')
        .map_or(value, |(principal_id, _)| principal_id);
    parse_uuid(principal_id, "subject_principal_id")
}

async fn ensure_not_last_owner(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    principal_id: Uuid,
) -> Result<(), Status> {
    let ownerless_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM tenant_memberships current_owner
        WHERE current_owner.tenant_id = $1
          AND current_owner.principal_id = $2
          AND current_owner.role = 'owner'
          AND current_owner.status = 'active'
          AND NOT EXISTS (
            SELECT 1
            FROM tenant_memberships other_owner
            WHERE other_owner.tenant_id = current_owner.tenant_id
              AND other_owner.principal_id <> $2
              AND other_owner.role = 'owner'
              AND other_owner.status = 'active'
          )
        "#,
    )
    .bind(tenant_id)
    .bind(principal_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(sql_status)?;
    ensure_ownerless_count_is_zero(ownerless_count)
}

async fn ensure_not_last_workspace_owner(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    principal_id: Uuid,
    workspace_id: Uuid,
) -> Result<(), Status> {
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
    ensure_ownerless_count_is_zero(ownerless_count)
}

fn ensure_ownerless_count_is_zero(ownerless_count: i64) -> Result<(), Status> {
    if ownerless_count > 0 {
        return Err(Status::failed_precondition(
            "every workspace must retain at least one active owner",
        ));
    }
    Ok(())
}

fn applied(target_id: Option<Uuid>) -> AppliedRevocation {
    AppliedRevocation { target_id }
}

#[cfg(test)]
mod tests {
    use tonic::Code;

    use super::ensure_ownerless_count_is_zero;

    #[test]
    fn last_owner_guard_allows_when_no_workspace_would_be_ownerless() {
        ensure_ownerless_count_is_zero(0).expect("no ownerless workspaces should pass");
    }

    #[test]
    fn last_owner_guard_blocks_ownerless_workspace() {
        let err = ensure_ownerless_count_is_zero(1).expect_err("ownerless workspace should fail");

        assert_eq!(err.code(), Code::FailedPrecondition);
    }
}
