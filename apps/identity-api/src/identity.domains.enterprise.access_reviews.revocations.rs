use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use super::db::AccessReviewItemRow;
use crate::domains::authz::AdminScope;
use crate::http::error::AppError;

#[derive(Debug, Clone)]
pub struct AppliedRevocation {
    pub target_type: &'static str,
    pub target_id: Option<Uuid>,
    pub oauth_client: Option<RevokedOAuthClient>,
}

#[derive(Debug, Clone)]
pub struct RevokedOAuthClient {
    pub id: Uuid,
    pub client_id: String,
}

pub async fn apply_revocation(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    item: &AccessReviewItemRow,
) -> Result<AppliedRevocation, AppError> {
    match item.item_type.as_str() {
        "member" => revoke_member(tx, tenant_id, item).await,
        "role" => revoke_role(tx, tenant_id, item).await,
        "service_account" => revoke_service_account(tx, tenant_id, item).await,
        "oauth_client" => revoke_oauth_client(tx, tenant_id, item).await,
        _ => Err(AppError::bad_request(
            "unsupported_access_review_item",
            "This access review item cannot be remediated.",
        )),
    }
}

async fn revoke_member(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    item: &AccessReviewItemRow,
) -> Result<AppliedRevocation, AppError> {
    let principal_id = parse_uuid_subject(&item.subject_id)?;
    crate::domains::enterprise::db::lock_tenant_owner_changes(tx, tenant_id).await?;
    ensure_not_last_owner(tx, tenant_id, principal_id).await?;
    crate::domains::enterprise::db::set_tenant_memberships_status(
        tx,
        tenant_id,
        principal_id,
        "removed",
        None,
        AdminScope::Tenant,
    )
    .await?;
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
    .await?;
    Ok(applied("principal", Some(principal_id), None))
}

async fn revoke_role(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    item: &AccessReviewItemRow,
) -> Result<AppliedRevocation, AppError> {
    let principal_id = parse_role_subject(&item.subject_id)?;
    let workspace_id = item.workspace_id.ok_or_else(|| {
        AppError::bad_request(
            "invalid_access_review_item",
            "Role review item is missing workspace scope.",
        )
    })?;
    crate::domains::enterprise::db::lock_tenant_owner_changes(tx, tenant_id).await?;
    ensure_not_last_workspace_owner(tx, tenant_id, principal_id, workspace_id).await?;
    sqlx::query(
        "UPDATE workspace_memberships SET status = 'removed', updated_at = NOW() WHERE workspace_id = $1 AND principal_id = $2",
    )
    .bind(workspace_id)
    .bind(principal_id)
    .execute(&mut **tx)
    .await?;
    Ok(applied("workspace_membership", Some(principal_id), None))
}

async fn revoke_service_account(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    item: &AccessReviewItemRow,
) -> Result<AppliedRevocation, AppError> {
    let principal_id = parse_uuid_subject(&item.subject_id)?;
    if let Some(workspace_id) = item.workspace_id {
        crate::domains::enterprise::db::lock_tenant_owner_changes(tx, tenant_id).await?;
        ensure_not_last_workspace_owner(tx, tenant_id, principal_id, workspace_id).await?;
    }
    sqlx::query("UPDATE principals SET status = 'revoked', updated_at = NOW() WHERE id = $1 AND tenant_id = $2")
        .bind(principal_id)
        .bind(tenant_id)
        .execute(&mut **tx)
        .await?;
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
    .await?;
    Ok(applied("service_account", Some(principal_id), None))
}

async fn revoke_oauth_client(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    item: &AccessReviewItemRow,
) -> Result<AppliedRevocation, AppError> {
    let client = sqlx::query_as::<_, (Uuid, String)>(
        r#"
        UPDATE oauth_clients
        SET revoked_at = NOW(), updated_at = NOW()
        WHERE tenant_id = $1 AND client_id = $2 AND revoked_at IS NULL
        RETURNING id, client_id
        "#,
    )
    .bind(tenant_id)
    .bind(&item.subject_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| {
        AppError::not_found(
            "oauth_client_not_found",
            "OAuth client not found or already revoked.",
        )
    })?;
    Ok(applied(
        "oauth_client",
        Some(client.0),
        Some(RevokedOAuthClient {
            id: client.0,
            client_id: client.1,
        }),
    ))
}

fn applied(
    target_type: &'static str,
    target_id: Option<Uuid>,
    oauth_client: Option<RevokedOAuthClient>,
) -> AppliedRevocation {
    AppliedRevocation {
        target_type,
        target_id,
        oauth_client,
    }
}

async fn ensure_not_last_owner(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    principal_id: Uuid,
) -> Result<(), AppError> {
    let ownerless_count = crate::domains::enterprise::db::ownerless_workspace_count_after_status(
        tx,
        tenant_id,
        principal_id,
        AdminScope::Tenant,
    )
    .await?;
    ensure_ownerless_count_is_zero(ownerless_count)
}

async fn ensure_not_last_workspace_owner(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    principal_id: Uuid,
    workspace_id: Uuid,
) -> Result<(), AppError> {
    let ownerless = sqlx::query_scalar::<_, i64>(
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
            SELECT 1 FROM workspace_memberships other_owner
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
    .await?;
    ensure_ownerless_count_is_zero(ownerless)
}

fn ensure_ownerless_count_is_zero(ownerless_count: i64) -> Result<(), AppError> {
    if ownerless_count > 0 {
        return Err(AppError::conflict(
            "last_owner_removal",
            "Every workspace must retain at least one active owner.",
        ));
    }
    Ok(())
}

fn parse_uuid_subject(value: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(value).map_err(|_| {
        AppError::bad_request(
            "invalid_access_review_item",
            "Access review item subject is invalid.",
        )
    })
}

fn parse_role_subject(value: &str) -> Result<Uuid, AppError> {
    let principal_id = value
        .split_once(':')
        .map_or(value, |(principal_id, _)| principal_id);
    parse_uuid_subject(principal_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_role_subject_accepts_snapshot_subject_with_workspace_suffix() {
        let principal_id = Uuid::new_v4();
        let workspace_id = Uuid::new_v4();
        let subject = format!("{principal_id}:{workspace_id}");

        assert_eq!(parse_role_subject(&subject).unwrap(), principal_id);
    }

    #[test]
    fn parse_role_subject_accepts_plain_principal_id() {
        let principal_id = Uuid::new_v4();

        assert_eq!(
            parse_role_subject(&principal_id.to_string()).unwrap(),
            principal_id
        );
    }

    #[test]
    fn parse_uuid_subject_rejects_invalid_subjects() {
        let err = parse_uuid_subject("not-a-uuid").expect_err("invalid subjects must fail");

        assert_eq!(err.code, "invalid_access_review_item");
    }
}
