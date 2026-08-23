use super::types::{InvitationView, MemberView};
use crate::http::error::AppError;
use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

use super::pagination::{InvitationListCursor, MemberListCursor};

#[derive(Debug, FromRow)]
pub struct MemberRecord {
    pub user_id: Uuid,
    pub email: String,
    pub display_name: String,
    pub role: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl MemberRecord {
    pub fn into_view(self) -> MemberView {
        MemberView {
            user_id: self.user_id,
            email: self.email,
            display_name: self.display_name,
            role: self.role,
            status: self.status,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

#[derive(Debug, FromRow)]
pub struct InvitationRecord {
    pub id: Uuid,
    pub email: String,
    pub role: String,
    pub status: String,
    pub expires_at: DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl InvitationRecord {
    pub fn into_view(self) -> InvitationView {
        InvitationView {
            id: self.id,
            email: self.email,
            role: self.role,
            status: self.status,
            expires_at: self.expires_at,
            accepted_at: self.accepted_at,
            revoked_at: self.revoked_at,
            created_at: self.created_at,
        }
    }
}

pub struct AuditEventInput<'a> {
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
    let workspace_id = input.workspace_id.ok_or_else(|| {
        AppError::internal("missing_workspace", "Workspace audit requires a workspace.")
    })?;
    crate::domains::audit::record_event_tx(
        tx,
        crate::domains::audit::AuditRecordInput {
            workspace_id,
            actor_user_id: input.actor_principal_id,
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
}

pub async fn revoke_user_sessions(
    redis: &nvbes_redis::RedisPool,
    user_id: Uuid,
) -> Result<(), AppError> {
    nvbes_redis::session::clear_user_sessions(redis, &user_id.to_string())
        .await
        .map_err(|err| AppError::internal("redis_session_revoke_failed", err.to_string()))
}

pub(super) async fn list_members(
    pool: &sqlx::PgPool,
    workspace_id: Uuid,
    cursor: Option<&MemberListCursor>,
    role: Option<&str>,
    email_prefix: Option<&str>,
    limit: i64,
) -> Result<Vec<MemberRecord>, AppError> {
    sqlx::query_as::<_, MemberRecord>(
        r#"
        SELECT
          wm.user_id,
          u.email,
          u.display_name,
          wm.role::text AS role,
          wm.status::text AS status,
          wm.created_at,
          wm.updated_at
        FROM workspace_memberships wm
        INNER JOIN users u ON u.id = wm.user_id
        WHERE wm.workspace_id = $1
          AND (
            $2::smallint IS NULL
            OR (
              CASE wm.role
                WHEN 'owner' THEN 0
                WHEN 'admin' THEN 1
                WHEN 'member' THEN 4
                WHEN 'viewer' THEN 5
                ELSE 6
              END,
              lower(u.email),
              wm.user_id
            ) > ($2, $3, $4)
          )
          AND ($5::text IS NULL OR wm.role::text = $5)
          AND ($6::text IS NULL OR lower(u.email) LIKE $6 || '%' ESCAPE '\')
        ORDER BY
          CASE wm.role
            WHEN 'owner' THEN 0
            WHEN 'admin' THEN 1
            WHEN 'member' THEN 4
            WHEN 'viewer' THEN 5
            ELSE 6
          END,
          lower(u.email) ASC,
          wm.user_id ASC
        LIMIT $7
        "#,
    )
    .bind(workspace_id)
    .bind(cursor.map(|value| value.role_rank))
    .bind(cursor.map(|value| value.normalized_email.as_str()))
    .bind(cursor.map(|value| value.user_id))
    .bind(role)
    .bind(email_prefix)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}

pub(super) async fn list_invitations(
    pool: &sqlx::PgPool,
    workspace_id: Uuid,
    cursor: Option<&InvitationListCursor>,
    statuses: &[String],
    limit: i64,
) -> Result<Vec<InvitationRecord>, AppError> {
    sqlx::query_as::<_, InvitationRecord>(
        r#"
        SELECT
          id,
          email,
          role::text AS role,
          status::text AS status,
          expires_at,
          accepted_at,
          revoked_at,
          created_at
        FROM workspace_invitations
        WHERE workspace_id = $1
          AND (
            $2::timestamp with time zone IS NULL
            OR (created_at, id) < ($2, $3)
          )
          AND (cardinality($4::text[]) = 0 OR status::text = ANY($4))
        ORDER BY created_at DESC, id DESC
        LIMIT $5
        "#,
    )
    .bind(workspace_id)
    .bind(cursor.map(|value| value.created_at))
    .bind(cursor.map(|value| value.id))
    .bind(statuses)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}

pub async fn fetch_active_member_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: Uuid,
    member_id: Uuid,
) -> Result<Option<MemberRecord>, AppError> {
    sqlx::query_as::<_, MemberRecord>(
        r#"
        SELECT
          wm.user_id,
          u.email,
          u.display_name,
          wm.role::text AS role,
          wm.status::text AS status,
          wm.created_at,
          wm.updated_at
        FROM workspace_memberships wm
        INNER JOIN users u ON u.id = wm.user_id
        WHERE wm.workspace_id = $1
          AND wm.user_id = $2
          AND wm.status = 'active'
        "#,
    )
    .bind(workspace_id)
    .bind(member_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(Into::into)
}

pub async fn update_member_role_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: Uuid,
    member_id: Uuid,
    role: &str,
) -> Result<MemberRecord, AppError> {
    sqlx::query_as::<_, MemberRecord>(
        r#"
        UPDATE workspace_memberships
        SET role = $3::workspace_member_role,
            updated_at = NOW()
        WHERE workspace_id = $1
          AND user_id = $2
        RETURNING
          user_id,
          (
            SELECT email FROM users WHERE id = workspace_memberships.user_id
          ) AS email,
          (
            SELECT name FROM users WHERE id = workspace_memberships.user_id
          ) AS name,
          role::text AS role,
          status::text AS status,
          created_at,
          updated_at
        "#,
    )
    .bind(workspace_id)
    .bind(member_id)
    .bind(role)
    .fetch_one(&mut **tx)
    .await
    .map_err(Into::into)
}

pub async fn remove_member_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: Uuid,
    member_id: Uuid,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE workspace_memberships
        SET status = 'removed',
            updated_at = NOW()
        WHERE workspace_id = $1
          AND user_id = $2
        "#,
    )
    .bind(workspace_id)
    .bind(member_id)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub async fn fetch_current_user_email(
    db: &sqlx::PgPool,
    user_id: Uuid,
) -> Result<String, AppError> {
    sqlx::query_scalar::<_, String>(
        r#"
        SELECT email
        FROM users
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .fetch_one(db)
    .await
    .map_err(Into::into)
}
