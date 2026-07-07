use nvbes_product_account::cloud_boundary::UpdateWorkspaceMembershipStatusCommand;
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

#[expect(
    clippy::too_many_arguments,
    reason = "Status update keeps persisted states and audit metadata explicit."
)]
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

    crate::domains::cloud::workspace_port::update_workspace_membership_status_tx(
        &mut tx,
        &UpdateWorkspaceMembershipStatusCommand {
            actor_principal_id: access.auth.user_id,
            workspace_id: access.workspace_id,
            principal_id: service_account_id,
            status: membership_status.to_string(),
        },
    )
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
