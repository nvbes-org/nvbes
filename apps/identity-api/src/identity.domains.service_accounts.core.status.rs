use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domains::{
        authz::WorkspaceAccess,
        service_accounts::{
            audit::record_audit_event, core::get_service_account,
            policy::enforce_target_role_management, types::ServiceAccountView,
        },
    },
    http::error::AppError,
};

pub async fn suspend_service_account(
    db: &PgPool,
    access: &WorkspaceAccess,
    service_account_id: Uuid,
    ip: Option<&str>,
    user_agent: Option<&str>,
) -> Result<ServiceAccountView, AppError> {
    update_service_account_status(
        db,
        access,
        service_account_id,
        "suspended",
        "suspended",
        "service_account.suspended",
        ip,
        user_agent,
    )
    .await
}

pub async fn reactivate_service_account(
    db: &PgPool,
    access: &WorkspaceAccess,
    service_account_id: Uuid,
    ip: Option<&str>,
    user_agent: Option<&str>,
) -> Result<ServiceAccountView, AppError> {
    update_service_account_status(
        db,
        access,
        service_account_id,
        "active",
        "active",
        "service_account.reactivated",
        ip,
        user_agent,
    )
    .await
}

async fn update_service_account_status(
    db: &PgPool,
    access: &WorkspaceAccess,
    service_account_id: Uuid,
    principal_status: &str,
    membership_status: &str,
    audit_action: &str,
    ip: Option<&str>,
    user_agent: Option<&str>,
) -> Result<ServiceAccountView, AppError> {
    let current = get_service_account(db, access, service_account_id).await?;
    enforce_target_role_management(access.role, &current.role)?;

    let mut tx = db.begin().await?;
    sqlx::query(
        r#"
        UPDATE principals
        SET status = $2::principal_status,
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(service_account_id)
    .bind(principal_status)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        UPDATE workspace_memberships
        SET status = $3::workspace_member_status,
            updated_at = NOW()
        WHERE workspace_id = $1
          AND principal_id = $2
        "#,
    )
    .bind(access.workspace_id)
    .bind(service_account_id)
    .bind(membership_status)
    .execute(&mut *tx)
    .await?;

    record_audit_event(
        &mut tx,
        access,
        audit_action,
        Some(service_account_id),
        serde_json::json!({
            "principal_status": principal_status,
            "membership_status": membership_status,
        }),
        ip,
        user_agent,
    )
    .await?;

    tx.commit().await?;
    get_service_account(db, access, service_account_id).await
}
