use chrono::{Duration, Utc};
use sqlx::Row;
use uuid::Uuid;

use super::db::{self, WorkspaceRecord};
use super::types::*;
use super::validation::*;
use crate::{domains::auth::types::AuthContext, http::error::AppError};
use sqlx::PgPool;

pub async fn list_workspaces(
    db: &PgPool,
    auth: &AuthContext,
) -> Result<WorkspaceListResponse, AppError> {
    let rows = sqlx::query_as::<_, WorkspaceRecord>(
        r#"
        SELECT
          w.id,
          COALESCE(
            w.owner_user_id,
            (SELECT principal_id FROM workspace_memberships WHERE workspace_id = w.id AND role = 'admin' LIMIT 1),
            '00000000-0000-0000-0000-000000000000'::uuid
          ) AS owner_principal_id,
          w.name,
          w.workspace_type::text AS workspace_type,
          w.data_region::text AS data_region,
          w.jurisdiction::text AS jurisdiction,
          wm.role::text AS role,
          p.code AS plan_code,
          w.trial_ends_at,
          wp.member_can_create_share_links,
          wp.require_admin_approval_for_member_share,
          wp.default_share_link_ttl_days,
          wp.max_share_link_ttl_days,
          w.created_at,
          w.updated_at
        FROM workspace_memberships wm
        INNER JOIN workspaces w ON w.id = wm.workspace_id
        INNER JOIN plans p ON p.code = w.plan_code
        INNER JOIN workspace_policies wp ON wp.workspace_id = w.id
        WHERE wm.principal_id = $1
          AND wm.status = 'active'
        ORDER BY w.created_at ASC
        "#,
    )
    .bind(auth.user_id)
    .fetch_all(db)
    .await?;

    Ok(WorkspaceListResponse {
        workspaces: rows.into_iter().map(WorkspaceRecord::into_view).collect(),
    })
}

pub async fn create_workspace(
    db: &PgPool,
    auth: &AuthContext,
    input: CreateWorkspaceInput,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<WorkspaceResponse, AppError> {
    let name = validate_workspace_name(&input.name)?;
    let workspace_type = parse_workspace_type(input.workspace_type.as_deref())?;
    let tenant_id = auth
        .tenant_id
        .ok_or_else(|| AppError::forbidden("tenant_required", "Tenant context is required."))?;

    let region = input.region.as_deref().unwrap_or("eu");
    let jurisdiction = input.jurisdiction.as_deref().unwrap_or("gdpr");

    let now = Utc::now();
    let trial_ends_at = now + Duration::days(14);

    let mut tx = db.begin().await?;

    let plan =
        sqlx::query("SELECT id, max_share_link_ttl_days FROM plans WHERE code = 'trial' LIMIT 1")
            .fetch_one(&mut *tx)
            .await?;
    let plan_id: Uuid = plan.get("id");
    let plan_code = "trial".to_string();
    let max_share_link_ttl_days: i32 = plan.get("max_share_link_ttl_days");

    let workspace_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO workspaces (
          tenant_id,
          organization_id,
          workspace_type,
          name,
          plan_code,
          trial_ends_at,
          data_region,
          jurisdiction
        )
        VALUES ($1, NULL, $2::workspace_type, $3, $4, $5, $6::data_region, $7::legal_jurisdiction)
        RETURNING id
        "#,
    )
    .bind(tenant_id)
    .bind(workspace_type)
    .bind(&name)
    .bind(&plan_code)
    .bind(trial_ends_at)
    .bind(region)
    .bind(jurisdiction)
    .fetch_one(&mut *tx)
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
        "INSERT INTO workspace_memberships (workspace_id, principal_id, role, status, source) VALUES ($1, $2, 'owner', 'active', 'manual')",
    )
    .bind(workspace_id)
    .bind(auth.user_id)
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

    nvbes_audit::insert_audit_event_tx(
        &mut *tx,
        nvbes_audit::AuditEventInput {
            tenant_id,
            workspace_id: Some(workspace_id),
            actor_principal_id: Some(auth.user_id),
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

    db::get_workspace_by_id(db, workspace_id, "owner").await
}
