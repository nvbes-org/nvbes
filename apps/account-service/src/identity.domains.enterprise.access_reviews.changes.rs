use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use super::{db::AccessReviewItemRow, types::AccessReviewDecisionInput};
use crate::http::error::AppError;

#[derive(Debug, Clone)]
pub struct AppliedAccessChange {
    pub target_type: &'static str,
    pub target_id: Option<Uuid>,
    pub target_role: String,
}

pub async fn apply_change(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    item: &AccessReviewItemRow,
    input: &AccessReviewDecisionInput,
) -> Result<AppliedAccessChange, AppError> {
    let target_role = input
        .change
        .as_ref()
        .and_then(|change| change.target_role.as_deref())
        .map(str::trim)
        .ok_or_else(|| {
            AppError::bad_request(
                "invalid_access_review_change",
                "A target role is required for change decisions.",
            )
        })?;

    match item.item_type.as_str() {
        "role" => change_role(tx, tenant_id, item, target_role).await,
        "service_account" => change_service_account_role(tx, tenant_id, item, target_role).await,
        _ => Err(AppError::bad_request(
            "unsupported_access_review_change",
            "Only role and service account review items can apply role changes.",
        )),
    }
}

async fn change_role(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    item: &AccessReviewItemRow,
    target_role: &str,
) -> Result<AppliedAccessChange, AppError> {
    let principal_id = parse_role_subject(&item.subject_id)?;
    let workspace_id = item.workspace_id.ok_or_else(|| {
        AppError::bad_request(
            "invalid_access_review_item",
            "Role review item is missing workspace scope.",
        )
    })?;
    change_workspace_role(tx, tenant_id, workspace_id, principal_id, target_role).await?;
    Ok(applied(
        "workspace_membership",
        Some(principal_id),
        target_role,
    ))
}

async fn change_service_account_role(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    item: &AccessReviewItemRow,
    target_role: &str,
) -> Result<AppliedAccessChange, AppError> {
    let principal_id = parse_uuid_subject(&item.subject_id)?;
    let workspace_id = item.workspace_id.ok_or_else(|| {
        AppError::bad_request(
            "invalid_access_review_item",
            "Service account review item is missing workspace scope.",
        )
    })?;
    change_workspace_role(tx, tenant_id, workspace_id, principal_id, target_role).await?;
    Ok(applied("service_account", Some(principal_id), target_role))
}

async fn change_workspace_role(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    workspace_id: Uuid,
    principal_id: Uuid,
    target_role: &str,
) -> Result<(), AppError> {
    crate::domains::enterprise::db::lock_tenant_owner_changes(tx, tenant_id).await?;
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
    .await?
    .rows_affected();

    if rows == 0 {
        return Err(AppError::not_found(
            "workspace_membership_not_found",
            "Workspace membership not found for access review change.",
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
) -> Result<(), AppError> {
    if target_role == "owner" {
        return Ok(());
    }
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
    if ownerless > 0 {
        return Err(AppError::conflict(
            "last_owner_removal",
            "Every workspace must retain at least one active owner.",
        ));
    }
    Ok(())
}

fn applied(
    target_type: &'static str,
    target_id: Option<Uuid>,
    target_role: &str,
) -> AppliedAccessChange {
    AppliedAccessChange {
        target_type,
        target_id,
        target_role: target_role.to_string(),
    }
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
    fn parse_uuid_subject_rejects_invalid_subjects() {
        let err = parse_uuid_subject("not-a-uuid").expect_err("invalid subjects must fail");

        assert_eq!(err.code, "invalid_access_review_item");
    }
}
