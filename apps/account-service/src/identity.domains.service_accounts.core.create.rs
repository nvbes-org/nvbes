use chrono::Utc;
use nvbes_product_account::cloud_boundary::UpsertWorkspaceMembershipCommand;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domains::{
        authz::WorkspaceAccess,
        service_accounts::{
            audit::record_audit_event,
            core::get_service_account,
            policy::{normalize_requested_role, require_name, required_tenant_id},
            types::{CreateServiceAccountInput, ServiceAccountView},
        },
    },
    http::error::AppError,
};

pub async fn create_service_account(
    db: &PgPool,
    access: &WorkspaceAccess,
    input: CreateServiceAccountInput,
    ip: Option<&str>,
    user_agent: Option<&str>,
) -> Result<ServiceAccountView, AppError> {
    let role = normalize_requested_role(access.role, input.role.as_deref())?;
    let name = require_name(&input.name)?;
    let mut tx = db.begin().await?;
    let principal_id = Uuid::new_v4();
    let tenant_id = required_tenant_id(access)?;
    let now = Utc::now();

    sqlx::query(
        r#"
        INSERT INTO principals (id, tenant_id, principal_kind, status, display_name, created_at, updated_at)
        VALUES ($1, $2, 'service_account', 'active', $3, $4, $4)
        "#,
    )
    .bind(principal_id)
    .bind(tenant_id)
    .bind(&name)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO tenant_memberships (tenant_id, principal_id, principal_kind, status, source, created_at, updated_at)
        VALUES ($1, $2, 'service_account', 'active', 'system', $3, $3)
        "#,
    )
    .bind(tenant_id)
    .bind(principal_id)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    if let Some(organization_id) = access.organization_id {
        sqlx::query(
            r#"
            INSERT INTO organization_memberships (organization_id, principal_id, status, source, created_at, updated_at)
            VALUES ($1, $2, 'active', 'system', $3, $3)
            ON CONFLICT (organization_id, principal_id) DO NOTHING
            "#,
        )
        .bind(organization_id)
        .bind(principal_id)
        .bind(now)
        .execute(&mut *tx)
        .await?;
    }

    sqlx::query(
        r#"
        INSERT INTO service_accounts (
          principal_id, tenant_id, workspace_id, created_by_principal_id, name, description,
          auth_method, client_id, secret_hash, public_key_jwk, last_rotated_at, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, 'oauth_client_credentials', NULL, NULL, NULL, NULL, $7, $7)
        "#,
    )
    .bind(principal_id)
    .bind(tenant_id)
    .bind(access.workspace_id)
    .bind(access.auth.user_id)
    .bind(&name)
    .bind(input.description.as_deref())
    .bind(now)
    .execute(&mut *tx)
    .await?;

    crate::domains::cloud::workspace_port::upsert_workspace_membership_tx(
        &mut tx,
        &UpsertWorkspaceMembershipCommand {
            actor_principal_id: access.auth.user_id,
            workspace_id: access.workspace_id,
            principal_id,
            role: role.to_string(),
            status: "active".to_string(),
            source: "system".to_string(),
        },
    )
    .await?;

    record_audit_event(
        &mut tx,
        access,
        "service_account.created",
        Some(principal_id),
        serde_json::json!({
            "service_account_principal_id": principal_id,
            "role": role.to_string(),
            "name": name,
            "description": input.description,
        }),
        ip,
        user_agent,
    )
    .await?;

    tx.commit().await?;
    get_service_account(db, access, principal_id).await
}
