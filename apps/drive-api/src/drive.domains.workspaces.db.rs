use chrono::{DateTime, Utc};
use sqlx::{FromRow, Row};
use uuid::Uuid;

use super::types::{WorkspacePolicyView, WorkspaceView};
use crate::http::error::AppError;
use sqlx::PgPool;

#[derive(Debug, FromRow)]
pub struct WorkspaceRecord {
    pub id: Uuid,
    pub name: String,
    pub workspace_type: String,
    pub role: String,
    pub plan_code: String,
    pub owner_principal_id: Uuid,
    pub trial_ends_at: Option<DateTime<Utc>>,
    pub member_can_create_share_links: bool,
    pub require_admin_approval_for_member_share: bool,
    pub default_share_link_ttl_days: i32,
    pub max_share_link_ttl_days: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl WorkspaceRecord {
    pub fn into_view(self) -> WorkspaceView {
        WorkspaceView {
            id: self.id,
            name: self.name,
            workspace_type: self.workspace_type,
            role: self.role,
            plan_code: self.plan_code,
            owner_principal_id: self.owner_principal_id,
            trial_ends_at: self.trial_ends_at,
            policy: WorkspacePolicyView {
                member_can_create_share_links: self.member_can_create_share_links,
                require_admin_approval_for_member_share: self
                    .require_admin_approval_for_member_share,
                default_share_link_ttl_days: self.default_share_link_ttl_days,
                max_share_link_ttl_days: self.max_share_link_ttl_days,
            },
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

pub struct CurrentWorkspaceRecord {
    pub name: String,
    pub updated_at: DateTime<Utc>,
    pub plan_max_share_link_ttl_days: i32,
    pub member_can_create_share_links: bool,
    pub require_admin_approval_for_member_share: bool,
    pub default_share_link_ttl_days: i32,
    pub max_share_link_ttl_days: i32,
}

pub async fn list_workspaces(
    db: &PgPool,
    user_id: Uuid,
    tenant_id: Option<Uuid>,
) -> Result<Vec<WorkspaceRecord>, AppError> {
    let rows = sqlx::query_as::<_, WorkspaceRecord>(
        r#"
        SELECT
          w.id,
          w.name,
          w.workspace_type::text AS workspace_type,
          wm.role::text AS role,
          p.code AS plan_code,
          w.owner_principal_id,
          w.trial_ends_at,
          wp.member_can_create_share_links,
          wp.require_admin_approval_for_member_share,
          wp.default_share_link_ttl_days,
          wp.max_share_link_ttl_days,
          w.created_at,
          w.updated_at
        FROM workspace_memberships wm
        INNER JOIN workspaces w ON w.id = wm.workspace_id
        INNER JOIN plans p ON p.id = w.plan_id
        INNER JOIN workspace_policies wp ON wp.workspace_id = w.id
        WHERE wm.user_id = $1
          AND wm.status = 'active'
          AND w.deleted_at IS NULL
          AND ($2::uuid IS NULL OR w.tenant_id = $2)
        ORDER BY w.created_at ASC
        "#,
    )
    .bind(user_id)
    .bind(tenant_id)
    .fetch_all(db)
    .await?;

    Ok(rows)
}

pub async fn fetch_workspace_by_id(
    db: &PgPool,
    workspace_id: Uuid,
    role: &str,
) -> Result<WorkspaceRecord, AppError> {
    let row = sqlx::query_as::<_, WorkspaceRecord>(
        r#"
        SELECT
          w.id,
          w.name,
          w.workspace_type::text AS workspace_type,
          $2::text AS role,
          p.code AS plan_code,
          w.owner_principal_id,
          w.trial_ends_at,
          wp.member_can_create_share_links,
          wp.require_admin_approval_for_member_share,
          wp.default_share_link_ttl_days,
          wp.max_share_link_ttl_days,
          w.created_at,
          w.updated_at
        FROM workspaces w
        INNER JOIN plans p ON p.id = w.plan_id
        INNER JOIN workspace_policies wp ON wp.workspace_id = w.id
        WHERE w.id = $1
          AND w.deleted_at IS NULL
        "#,
    )
    .bind(workspace_id)
    .bind(role)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::not_found("workspace_not_found", "Workspace not found."))?;

    Ok(row)
}

pub async fn fetch_workspace_for_update(
    db: &PgPool,
    workspace_id: Uuid,
) -> Result<CurrentWorkspaceRecord, AppError> {
    let mut tx = db.begin().await?;
    let row = sqlx::query(
        r#"
        SELECT
          w.name,
          w.updated_at,
          p.max_share_link_ttl_days AS plan_max_share_link_ttl_days,
          wp.member_can_create_share_links,
          wp.require_admin_approval_for_member_share,
          wp.default_share_link_ttl_days,
          wp.max_share_link_ttl_days
        FROM workspaces w
        INNER JOIN plans p ON p.id = w.plan_id
        INNER JOIN workspace_policies wp ON wp.workspace_id = w.id
        WHERE w.id = $1
          AND w.deleted_at IS NULL
        FOR UPDATE OF w, wp
        "#,
    )
    .bind(workspace_id)
    .fetch_optional(&mut *tx)
    .await?;

    tx.commit().await?;

    let row =
        row.ok_or_else(|| AppError::not_found("workspace_not_found", "Workspace not found."))?;
    Ok(CurrentWorkspaceRecord {
        name: row.get("name"),
        updated_at: row.get("updated_at"),
        plan_max_share_link_ttl_days: row.get("plan_max_share_link_ttl_days"),
        member_can_create_share_links: row.get("member_can_create_share_links"),
        require_admin_approval_for_member_share: row.get("require_admin_approval_for_member_share"),
        default_share_link_ttl_days: row.get("default_share_link_ttl_days"),
        max_share_link_ttl_days: row.get("max_share_link_ttl_days"),
    })
}

pub struct AuditEventInput<'a> {
    pub tenant_id: Uuid,
    pub workspace_id: Option<Uuid>,
    pub actor_principal_id: Option<Uuid>,
    pub action: &'a str,
    pub target_type: &'a str,
    pub target_id: Option<Uuid>,
    pub ip: Option<&'a str>,
    pub user_agent: Option<&'a str>,
    pub metadata: serde_json::Value,
}

pub async fn insert_audit_event(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    input: AuditEventInput<'_>,
) -> Result<(), AppError> {
    nvbes_audit::insert_audit_event_tx(
        &mut **tx,
        nvbes_audit::AuditEventInput {
            tenant_id: input.tenant_id,
            workspace_id: input.workspace_id,
            actor_principal_id: input.actor_principal_id,
            action: input.action,
            target_type: input.target_type,
            target_id: input.target_id,
            ip: input.ip,
            user_agent: input.user_agent,
            metadata: input.metadata,
        },
    )
    .await
    .map_err(AppError::from)
}
