use chrono::{DateTime, Utc};
use sqlx::{FromRow, Row};
use uuid::Uuid;

use super::types::*;
use crate::http::error::AppError;
use sqlx::PgPool;

#[derive(Debug, FromRow)]
pub struct WorkspaceRecord {
    pub id: Uuid,
    pub owner_principal_id: Uuid,
    pub name: String,
    pub workspace_type: String,
    pub data_region: String,
    pub jurisdiction: String,
    pub role: String,
    pub plan_code: String,
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
            owner_principal_id: self.owner_principal_id,
            name: self.name,
            workspace_type: self.workspace_type,
            data_region: self.data_region,
            jurisdiction: self.jurisdiction,
            role: self.role,
            plan_code: self.plan_code,
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

pub async fn get_workspace_by_id(
    db: &PgPool,
    workspace_id: Uuid,
    role: &str,
) -> Result<WorkspaceResponse, AppError> {
    let row = sqlx::query_as::<_, WorkspaceRecord>(
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
          $2::text AS role,
          p.code AS plan_code,
          w.trial_ends_at,
          wp.member_can_create_share_links,
          wp.require_admin_approval_for_member_share,
          wp.default_share_link_ttl_days,
          wp.max_share_link_ttl_days,
          w.created_at,
          w.updated_at
        FROM workspaces w
        INNER JOIN plans p ON p.code = w.plan_code
        INNER JOIN workspace_policies wp ON wp.workspace_id = w.id
        WHERE w.id = $1
        "#,
    )
    .bind(workspace_id)
    .bind(role)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::not_found("workspace_not_found", "Workspace not found."))?;

    Ok(WorkspaceResponse {
        workspace: row.into_view(),
    })
}

pub struct CurrentWorkspaceRecord {
    pub name: String,
    pub plan_max_share_link_ttl_days: i32,
    pub member_can_create_share_links: bool,
    pub require_admin_approval_for_member_share: bool,
    pub default_share_link_ttl_days: i32,
    pub max_share_link_ttl_days: i32,
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
          COALESCE(
            w.owner_user_id,
            (SELECT principal_id FROM workspace_memberships WHERE workspace_id = w.id AND role = 'admin' LIMIT 1),
            '00000000-0000-0000-0000-000000000000'::uuid
          ) AS owner_principal_id,
          p.max_share_link_ttl_days AS plan_max_share_link_ttl_days,
          wp.member_can_create_share_links,
          wp.require_admin_approval_for_member_share,
          wp.default_share_link_ttl_days,
          wp.max_share_link_ttl_days
        FROM workspaces w
        INNER JOIN plans p ON p.code = w.plan_code
        INNER JOIN workspace_policies wp ON wp.workspace_id = w.id
        WHERE w.id = $1
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
        plan_max_share_link_ttl_days: row.get("plan_max_share_link_ttl_days"),
        member_can_create_share_links: row.get("member_can_create_share_links"),
        require_admin_approval_for_member_share: row.get("require_admin_approval_for_member_share"),
        default_share_link_ttl_days: row.get("default_share_link_ttl_days"),
        max_share_link_ttl_days: row.get("max_share_link_ttl_days"),
    })
}
