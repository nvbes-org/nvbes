use tonic::Status;
use uuid::Uuid;

use crate::grpc::{
    pb::nvbes::enterprise::v1 as enterprise,
    service_status::{parse_uuid, sql_status},
};

#[path = "enterprise.grpc.user_access.audit.rs"]
mod audit;
#[path = "enterprise.grpc.user_access.db.rs"]
mod db;

#[cfg(test)]
#[path = "enterprise.grpc.user_access.contract_tests.rs"]
mod contract_tests;

pub async fn update_user_access(
    pool: &sqlx::PgPool,
    tenant_id: Uuid,
    actor_id: Uuid,
    request: enterprise::UpdateUserAccessRequest,
) -> Result<enterprise::UserAccessChange, Status> {
    let target_principal_id = parse_uuid(&request.target_principal_id, "target_principal_id")?;
    let scope = AccessScope::from_request(&request.scope, &request.organization_id)?;
    let role = validate_role(&request.role)?;
    let workspace_ids = parse_workspace_ids(&request.workspace_ids, true)?;

    let mut tx = pool.begin().await.map_err(sql_status)?;
    db::lock_tenant_owner_changes(&mut tx, tenant_id).await?;
    db::ensure_workspaces_belong(&mut tx, tenant_id, &workspace_ids, scope).await?;
    let current_role = db::target_role(&mut tx, tenant_id, target_principal_id, scope)
        .await?
        .ok_or_else(|| Status::not_found("enterprise user was not found"))?;
    let previous_workspace_ids =
        db::target_workspace_ids_for_lifecycle(&mut tx, tenant_id, target_principal_id, scope)
            .await?;
    db::ensure_not_last_owner_after_access(
        &mut tx,
        tenant_id,
        target_principal_id,
        &workspace_ids,
        role,
        scope,
    )
    .await?;
    db::replace_access(
        &mut tx,
        tenant_id,
        target_principal_id,
        &workspace_ids,
        role,
        scope,
    )
    .await?;
    let audit_workspace_ids = audit::merge_workspace_ids(&previous_workspace_ids, &workspace_ids);
    audit::insert_member_audit(
        &mut tx,
        tenant_id,
        scope,
        &audit_workspace_ids,
        actor_id,
        "enterprise.member.access_updated",
        target_principal_id,
        serde_json::json!({
            "before": {"role": current_role},
            "after": {"role": role, "workspace_ids": workspace_ids}
        }),
    )
    .await?;
    tx.commit().await.map_err(sql_status)?;
    Ok(access_change(
        tenant_id,
        target_principal_id,
        "active",
        workspace_ids,
    ))
}

pub async fn suspend_user_access(
    pool: &sqlx::PgPool,
    tenant_id: Uuid,
    actor_id: Uuid,
    request: enterprise::SuspendUserAccessRequest,
) -> Result<enterprise::UserAccessChange, Status> {
    let target_principal_id = parse_uuid(&request.target_principal_id, "target_principal_id")?;
    let scope = AccessScope::from_request(&request.scope, &request.organization_id)?;

    let mut tx = pool.begin().await.map_err(sql_status)?;
    db::lock_tenant_owner_changes(&mut tx, tenant_id).await?;
    let current_role = db::target_role(&mut tx, tenant_id, target_principal_id, scope)
        .await?
        .ok_or_else(|| Status::not_found("enterprise user was not found"))?;
    let audit_workspace_ids =
        db::target_workspace_ids_for_lifecycle(&mut tx, tenant_id, target_principal_id, scope)
            .await?;
    db::ensure_not_last_owner_after_status(&mut tx, tenant_id, target_principal_id, scope).await?;
    let changed = db::set_workspace_memberships_status(
        &mut tx,
        tenant_id,
        target_principal_id,
        "suspended",
        None,
        scope,
    )
    .await?;
    if changed == 0 {
        return Err(Status::not_found("enterprise user was not found"));
    }
    audit::insert_member_audit(
        &mut tx,
        tenant_id,
        scope,
        &audit_workspace_ids,
        actor_id,
        "enterprise.member.suspended",
        target_principal_id,
        serde_json::json!({"before": {"role": current_role}, "reason": request.reason}),
    )
    .await?;
    tx.commit().await.map_err(sql_status)?;
    Ok(access_change(
        tenant_id,
        target_principal_id,
        "suspended",
        audit_workspace_ids,
    ))
}

pub async fn reactivate_user_access(
    pool: &sqlx::PgPool,
    tenant_id: Uuid,
    actor_id: Uuid,
    request: enterprise::ReactivateUserAccessRequest,
) -> Result<enterprise::UserAccessChange, Status> {
    let target_principal_id = parse_uuid(&request.target_principal_id, "target_principal_id")?;
    let scope = AccessScope::from_request(&request.scope, &request.organization_id)?;
    let workspace_ids = parse_workspace_ids(&request.workspace_ids, false)?;

    let mut tx = pool.begin().await.map_err(sql_status)?;
    db::lock_tenant_owner_changes(&mut tx, tenant_id).await?;
    db::target_role_for_lifecycle(&mut tx, tenant_id, target_principal_id, scope)
        .await?
        .ok_or_else(|| Status::not_found("enterprise user was not found"))?;
    let previous_workspace_ids =
        db::target_workspace_ids_for_lifecycle(&mut tx, tenant_id, target_principal_id, scope)
            .await?;
    if !workspace_ids.is_empty() {
        db::ensure_workspaces_belong(&mut tx, tenant_id, &workspace_ids, scope).await?;
    }
    let changed = db::set_workspace_memberships_status(
        &mut tx,
        tenant_id,
        target_principal_id,
        "active",
        optional_workspace_ids(&workspace_ids),
        scope,
    )
    .await?;
    if changed == 0 {
        return Err(Status::not_found("enterprise user was not found"));
    }
    let audit_workspace_ids = audit::merge_workspace_ids(&previous_workspace_ids, &workspace_ids);
    audit::insert_member_audit(
        &mut tx,
        tenant_id,
        scope,
        &audit_workspace_ids,
        actor_id,
        "enterprise.member.reactivated",
        target_principal_id,
        serde_json::json!({"reason": request.reason, "workspace_ids": request.workspace_ids}),
    )
    .await?;
    tx.commit().await.map_err(sql_status)?;
    Ok(access_change(
        tenant_id,
        target_principal_id,
        "active",
        audit_workspace_ids,
    ))
}

#[derive(Clone, Copy)]
pub(crate) enum AccessScope {
    Tenant,
    Organization(Uuid),
}

impl AccessScope {
    pub(crate) fn from_request(scope: &str, organization_id: &str) -> Result<Self, Status> {
        match scope.trim() {
            "tenant" => Ok(Self::Tenant),
            "organization" => Ok(Self::Organization(parse_uuid(
                organization_id,
                "organization_id",
            )?)),
            _ => Err(Status::invalid_argument(
                "scope must be tenant or organization",
            )),
        }
    }

    pub(crate) fn organization_id(self) -> Option<Uuid> {
        match self {
            Self::Tenant => None,
            Self::Organization(organization_id) => Some(organization_id),
        }
    }
}

fn validate_role(role: &str) -> Result<&str, Status> {
    match role.trim() {
        "owner" | "admin" | "member" | "viewer" => Ok(role.trim()),
        _ => Err(Status::invalid_argument(
            "role must be owner, admin, member, or viewer",
        )),
    }
}

fn parse_workspace_ids(values: &[String], required: bool) -> Result<Vec<Uuid>, Status> {
    if required && values.is_empty() {
        return Err(Status::invalid_argument(
            "at least one workspace_id is required",
        ));
    }
    values
        .iter()
        .map(|value| parse_uuid(value, "workspace_id"))
        .collect()
}

fn optional_workspace_ids(workspace_ids: &[Uuid]) -> Option<&[Uuid]> {
    if workspace_ids.is_empty() {
        None
    } else {
        Some(workspace_ids)
    }
}

fn access_change(
    tenant_id: Uuid,
    target_principal_id: Uuid,
    status: &'static str,
    workspace_ids: Vec<Uuid>,
) -> enterprise::UserAccessChange {
    enterprise::UserAccessChange {
        tenant_id: tenant_id.to_string(),
        target_principal_id: target_principal_id.to_string(),
        status: status.to_string(),
        workspace_ids: workspace_ids
            .into_iter()
            .map(|workspace_id| workspace_id.to_string())
            .collect(),
    }
}
