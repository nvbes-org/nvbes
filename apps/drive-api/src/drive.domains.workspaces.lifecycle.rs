use chrono::{Duration, Utc};
use sqlx::Row;
use uuid::Uuid;

use super::db::{AuditEventInput, fetch_workspace_for_update, insert_audit_event};
use super::types::{CreateWorkspaceInput, UpdateWorkspaceInput, WorkspaceResponse};
use super::validation::{
    normalize_policy_update, parse_workspace_type, slugify, validate_workspace_name,
};
use crate::{
    domains::auth::types::AuthContext, domains::authz::WorkspaceAccess, http::error::AppError,
};

pub async fn create_workspace(
    db: &sqlx::PgPool,
    auth: &AuthContext,
    input: CreateWorkspaceInput,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<WorkspaceResponse, AppError> {
    let name = validate_workspace_name(&input.name)?;
    let workspace_type = parse_workspace_type(input.workspace_type.as_deref())?;
    let now = Utc::now();
    let trial_ends_at = now + Duration::days(14);
    let tenant_slug = format!("{}-{}", slugify(&name), Uuid::new_v4().simple());
    let tenant_id = input.tenant_id;
    let workspace_id = input.id;

    let mut tx = db.begin().await?;

    let plan =
        sqlx::query("SELECT id, max_share_link_ttl_days FROM plans WHERE code = 'trial' LIMIT 1")
            .fetch_one(&mut *tx)
            .await?;
    let plan_id: Uuid = plan.get("id");
    let max_share_link_ttl_days: i32 = plan.get("max_share_link_ttl_days");

    sqlx::query(
        r#"
        INSERT INTO tenants (id, kind, name, slug)
        VALUES ($1, 'personal', $2, $3)
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(tenant_id)
    .bind(&name)
    .bind(&tenant_slug)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO tenant_memberships (tenant_id, user_id, member_type, status, source)
        VALUES ($1, $2, 'user', 'active', 'manual')
        ON CONFLICT (tenant_id, user_id) DO NOTHING
        "#,
    )
    .bind(tenant_id)
    .bind(auth.user_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO workspaces (
          id,
          tenant_id,
          organization_id,
          workspace_type,
          name,
          owner_user_id,
          owner_principal_id,
          plan_id,
          trial_started_at,
          trial_ends_at
        )
        VALUES ($1, $2, $3, $4::workspace_type, $5, $6, $7, $8, $9, $10)
        "#,
    )
    .bind(workspace_id)
    .bind(tenant_id)
    .bind(auth.organization_id)
    .bind(workspace_type)
    .bind(&name)
    .bind(auth.user_id)
    .bind(auth.principal_id)
    .bind(plan_id)
    .bind(now)
    .bind(trial_ends_at)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO workspace_policies (
          workspace_id,
          member_can_create_share_links,
          require_admin_approval_for_member_share,
          default_share_link_ttl_days,
          max_share_link_ttl_days
        )
        VALUES ($1, FALSE, TRUE, 7, $2)
        "#,
    )
    .bind(workspace_id)
    .bind(max_share_link_ttl_days)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO workspace_memberships (workspace_id, user_id, role, status, source) VALUES ($1, $2, 'owner', 'active', 'manual')",
    )
    .bind(workspace_id)
    .bind(auth.user_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query("INSERT INTO quota_usage (workspace_id) VALUES ($1)")
        .bind(workspace_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query(
        r#"
        INSERT INTO subscriptions (
          workspace_id,
          plan_id,
          status,
          billing_provider,
          current_period_start,
          current_period_end
        )
        VALUES ($1, $2, 'trialing', 'stripe', $3, $4)
        "#,
    )
    .bind(workspace_id)
    .bind(plan_id)
    .bind(now)
    .bind(trial_ends_at)
    .execute(&mut *tx)
    .await?;

    insert_audit_event(
        &mut tx,
        AuditEventInput {
            tenant_id,
            workspace_id: Some(workspace_id),
            actor_principal_id: Some(auth.principal_id),
            action: "workspace.created",
            target_type: "workspace",
            target_id: Some(workspace_id),
            ip: ip.as_deref(),
            user_agent: user_agent.as_deref(),
            metadata: serde_json::json!({
                "workspace_type": workspace_type,
                "plan_code": "trial",
                "trial_days": 14,
            }),
        },
    )
    .await?;

    tx.commit().await?;

    super::service::get_workspace_by_id(db, workspace_id, "owner").await
}

pub async fn update_workspace(
    db: &sqlx::PgPool,
    access: &WorkspaceAccess,
    input: UpdateWorkspaceInput,
    expected_etag: Option<String>,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<WorkspaceResponse, AppError> {
    let current = fetch_workspace_for_update(db, access.workspace_id).await?;
    ensure_workspace_etag(
        expected_etag.as_deref(),
        access.workspace_id,
        current.updated_at,
    )?;
    let previous_name = current.name.clone();
    let name = match input.name {
        Some(name) => validate_workspace_name(&name)?,
        None => previous_name.clone(),
    };

    let policy = normalize_policy_update(&current, input.policy)?;
    let mut tx = crate::domains::authz::begin_workspace_transaction(db, access).await?;

    let updated = sqlx::query(
        r#"
        UPDATE workspaces
        SET name = $2,
            updated_at = NOW()
        WHERE id = $1
          AND deleted_at IS NULL
          AND updated_at = $3
        "#,
    )
    .bind(access.workspace_id)
    .bind(&name)
    .bind(current.updated_at)
    .execute(&mut *tx)
    .await?;

    if updated.rows_affected() != 1 {
        return Err(AppError::precondition_failed(
            "etag_mismatch",
            "Workspace has changed since the client last read it.",
        ));
    }

    sqlx::query(
        r#"
        UPDATE workspace_policies
        SET member_can_create_share_links = $2,
            require_admin_approval_for_member_share = $3,
            default_share_link_ttl_days = $4,
            max_share_link_ttl_days = $5,
            updated_at = NOW()
        WHERE workspace_id = $1
        "#,
    )
    .bind(access.workspace_id)
    .bind(policy.member_can_create_share_links)
    .bind(policy.require_admin_approval_for_member_share)
    .bind(policy.default_share_link_ttl_days)
    .bind(policy.max_share_link_ttl_days)
    .execute(&mut *tx)
    .await?;

    insert_audit_event(
        &mut tx,
        AuditEventInput {
            tenant_id: access.tenant_id.ok_or_else(|| AppError::internal("missing_tenant", "Tenant context is required."))?,
            workspace_id: Some(access.workspace_id),
            actor_principal_id: Some(access.auth.principal_id),
            action: "workspace.updated",
            target_type: "workspace",
            target_id: Some(access.workspace_id),
            ip: ip.as_deref(),
            user_agent: user_agent.as_deref(),
            metadata: serde_json::json!({
                "name_changed": previous_name != name,
                "policy": {
                    "member_can_create_share_links": policy.member_can_create_share_links,
                    "require_admin_approval_for_member_share": policy.require_admin_approval_for_member_share,
                    "default_share_link_ttl_days": policy.default_share_link_ttl_days,
                    "max_share_link_ttl_days": policy.max_share_link_ttl_days,
                }
            }),
        },
    )
    .await?;

    tx.commit().await?;

    super::service::get_workspace_by_id(
        db,
        access.workspace_id,
        nvbes_tenancy::role_as_str(access.role),
    )
    .await
}

fn ensure_workspace_etag(
    expected_etag: Option<&str>,
    workspace_id: uuid::Uuid,
    updated_at: chrono::DateTime<chrono::Utc>,
) -> Result<(), AppError> {
    let Some(expected_etag) = expected_etag else {
        return Ok(());
    };
    let current_etag = nvbes_core::http::etag::resource_etag("workspace", workspace_id, updated_at);
    if nvbes_core::http::etag::etag_list_matches(expected_etag, &current_etag) {
        return Ok(());
    }
    Err(AppError::precondition_failed(
        "etag_mismatch",
        "Workspace has changed since the client last read it.",
    ))
}
