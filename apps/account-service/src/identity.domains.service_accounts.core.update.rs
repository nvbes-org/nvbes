use nvbes_product_account::cloud_boundary::UpdateWorkspaceMembershipRoleCommand;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domains::{
        authz::WorkspaceAccess,
        service_accounts::{
            audit::record_audit_event,
            core::get_service_account,
            policy::{enforce_target_role_management, normalize_requested_role, require_name},
            types::{ServiceAccountView, UpdateServiceAccountInput},
        },
    },
    http::error::AppError,
};

pub async fn update_service_account(
    db: &PgPool,
    access: &WorkspaceAccess,
    service_account_id: Uuid,
    input: UpdateServiceAccountInput,
    ip: Option<&str>,
    user_agent: Option<&str>,
) -> Result<ServiceAccountView, AppError> {
    let current = get_service_account(db, access, service_account_id).await?;
    let next_role = match input.role.as_deref() {
        Some(role) => Some(normalize_requested_role(access.role, Some(role))?),
        None => None,
    };
    enforce_target_role_management(access.role, &current.role)?;
    let name = match input.name.as_deref() {
        Some(value) => Some(require_name(value)?),
        None => None,
    };

    let mut tx = db.begin().await?;
    sqlx::query(
        r#"
        UPDATE service_accounts
        SET name = COALESCE($2, name),
            description = COALESCE($3, description),
            updated_at = NOW()
        WHERE principal_id = $1
          AND workspace_id = $4
        "#,
    )
    .bind(service_account_id)
    .bind(name.as_deref())
    .bind(input.description.as_deref())
    .bind(access.workspace_id)
    .execute(&mut *tx)
    .await?;

    if let Some(role) = next_role {
        crate::domains::cloud::workspace_port::update_workspace_membership_role_tx(
            &mut tx,
            &UpdateWorkspaceMembershipRoleCommand {
                actor_principal_id: access.auth.user_id,
                workspace_id: access.workspace_id,
                principal_id: service_account_id,
                role: role.to_string(),
            },
        )
        .await?;
    }

    if let Some(display_name) = name.as_deref() {
        sqlx::query(
            r#"
            UPDATE principals
            SET display_name = $2,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(service_account_id)
        .bind(display_name)
        .execute(&mut *tx)
        .await?;
    }

    record_audit_event(
        &mut tx,
        access,
        "service_account.updated",
        Some(service_account_id),
        serde_json::json!({
            "name": name,
            "description": input.description,
            "role": input.role,
        }),
        ip,
        user_agent,
    )
    .await?;

    tx.commit().await?;
    get_service_account(db, access, service_account_id).await
}
