use crate::http::error::AppError;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

#[path = "identity.domains.auth.password.db.reset.rs"]
pub mod reset;

pub async fn find_principal_and_display_name_by_email(
    db: &PgPool,
    email: &str,
) -> Result<Option<(Uuid, String)>, AppError> {
    let row = sqlx::query(
        r#"SELECT principal_id, firstname, lastname, username FROM users WHERE lower(email) = lower($1) LIMIT 1"#,
    )
    .bind(email)
    .fetch_optional(db)
    .await?;

    if let Some(row) = row {
        use sqlx::Row;
        let firstname: Option<String> = row.get("firstname");
        let lastname: Option<String> = row.get("lastname");
        let username: Option<String> = row.get("username");
        Ok(Some((
            row.get("principal_id"),
            crate::domains::auth::types::derive_display_name(
                firstname.as_deref(),
                lastname.as_deref(),
                username.as_deref(),
            ),
        )))
    } else {
        Ok(None)
    }
}

pub async fn get_tenant_info_by_principal(
    db: &PgPool,
    principal_id: Uuid,
) -> Result<(Uuid, String), AppError> {
    let tenant_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT p.tenant_id
        FROM principals p
        WHERE p.id = $1
        LIMIT 1
        "#,
    )
    .bind(principal_id)
    .fetch_one(db)
    .await?;

    let tenant_kind: String = sqlx::query_scalar(
        r#"
        SELECT kind::text
        FROM tenants
        WHERE id = $1
        LIMIT 1
        "#,
    )
    .bind(tenant_id)
    .fetch_one(db)
    .await?;

    Ok((tenant_id, tenant_kind))
}

pub async fn insert_enterprise_recovery_request(
    db: &PgPool,
    id: Uuid,
    principal_id: Uuid,
    tenant_id: Uuid,
    email: &str,
    available_at: DateTime<Utc>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO enterprise_password_recovery_requests (
          id, principal_id, tenant_id, email, status, available_at, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, 'pending', $5, NOW(), NOW())
        ON CONFLICT (principal_id)
        DO UPDATE SET
          email = EXCLUDED.email,
          status = 'pending',
          available_at = EXCLUDED.available_at,
          approved_by_principal_id = NULL,
          approved_at = NULL,
          review_available_at = NULL,
          secondary_approved_by_principal_id = NULL,
          secondary_approved_at = NULL,
          reset_token_hash = NULL,
          reset_token_expires_at = NULL,
          consumed_at = NULL,
          updated_at = NOW()
        "#,
    )
    .bind(id)
    .bind(principal_id)
    .bind(tenant_id)
    .bind(email)
    .bind(available_at)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn find_recovery_request_for_approval(
    tx: &mut Transaction<'_, Postgres>,
    email: &str,
) -> Result<Option<sqlx::postgres::PgRow>, AppError> {
    let row = sqlx::query(
        r#"
        SELECT
          u.principal_id,
          p.tenant_id,
          t.kind::text AS tenant_kind,
          req.id AS request_id,
          req.available_at,
          req.approved_at,
          req.review_available_at,
          req.approved_by_principal_id,
          req.secondary_approved_by_principal_id,
          req.consumed_at,
          req.status
        FROM users u
        INNER JOIN principals p ON p.id = u.principal_id
        INNER JOIN tenants t ON t.id = p.tenant_id
        INNER JOIN enterprise_password_recovery_requests req ON req.principal_id = u.principal_id
        WHERE lower(u.email) = lower($1)
          AND req.status IN ('pending', 'first_approved')
        ORDER BY req.created_at DESC
        LIMIT 1
        FOR UPDATE
        "#,
    )
    .bind(email)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(row)
}

pub async fn check_approver_is_admin(
    db: &PgPool,
    tenant_id: Uuid,
    approver_principal_id: Uuid,
) -> Result<bool, AppError> {
    let is_admin = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
          SELECT 1
          FROM workspace_memberships wm
          INNER JOIN workspaces w ON w.id = wm.workspace_id
          WHERE w.tenant_id = $1
            AND wm.principal_id = $2
            AND wm.status = 'active'
            AND wm.role IN ('owner', 'admin')
        )
        "#,
    )
    .bind(tenant_id)
    .bind(approver_principal_id)
    .fetch_one(db)
    .await?;
    Ok(is_admin)
}

pub async fn update_recovery_first_approval(
    tx: &mut Transaction<'_, Postgres>,
    request_id: Uuid,
    approver_id: Uuid,
    review_available_at: DateTime<Utc>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE enterprise_password_recovery_requests
        SET status = 'first_approved',
            approved_by_principal_id = $2,
            approved_at = NOW(),
            review_available_at = $3,
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(request_id)
    .bind(approver_id)
    .bind(review_available_at)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn insert_audit_event(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    actor_id: Uuid,
    action: &str,
    target_id: Uuid,
    metadata: serde_json::Value,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO audit_events (
          tenant_id, workspace_id, actor_principal_id, action, target_type, target_id, metadata
        )
        VALUES ($1, NULL, $2, $3, 'user', $4, $5)
        "#,
    )
    .bind(tenant_id)
    .bind(actor_id)
    .bind(action)
    .bind(target_id)
    .bind(metadata)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn update_recovery_final_approval(
    tx: &mut Transaction<'_, Postgres>,
    request_id: Uuid,
    approver_id: Uuid,
    token_hash: &str,
    expires_at: DateTime<Utc>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE enterprise_password_recovery_requests
        SET status = 'approved',
            secondary_approved_by_principal_id = $2,
            secondary_approved_at = NOW(),
            reset_token_hash = $3,
            reset_token_expires_at = $4,
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(request_id)
    .bind(approver_id)
    .bind(token_hash)
    .bind(expires_at)
    .execute(&mut **tx)
    .await?;
    Ok(())
}
