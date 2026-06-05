use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domains::{
        authz::WorkspaceAccess,
        service_accounts::{
            helpers::{
                enforce_target_role_management, normalize_requested_role, now, record_audit_event,
                require_name, required_tenant_id, service_account_view_from_row,
            },
            types::{
                CreateServiceAccountInput, ServiceAccountView, ServiceAccountsResult,
                UpdateServiceAccountInput,
            },
        },
    },
    http::error::AppError,
};

pub async fn list_service_accounts(
    db: &PgPool,
    access: &WorkspaceAccess,
) -> Result<ServiceAccountsResult, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT
          sa.principal_id,
          sa.tenant_id,
          w.organization_id,
          sa.workspace_id,
          sa.name,
          sa.description,
          wm.role::text AS role,
          p.status::text AS principal_status,
          sa.created_at,
          sa.updated_at,
          oc.id AS oauth_client_uuid,
          oc.client_id,
          oc.name AS oauth_client_name,
          oc.created_at AS oauth_client_created_at,
          oc.revoked_at AS oauth_client_revoked_at,
          oc.client_assertion_required,
          oc.client_assertion_public_key_jwk IS NOT NULL AS client_assertion_public_key_configured,
          ocp.allowed_scopes,
          ocp.allowed_audiences,
          ocp.allowed_resources,
          ocp.required_acr::text AS required_acr
        FROM service_accounts sa
        INNER JOIN principals p ON p.id = sa.principal_id
        INNER JOIN workspaces w ON w.id = sa.workspace_id
        LEFT JOIN workspace_memberships wm
          ON wm.workspace_id = sa.workspace_id
         AND wm.principal_id = sa.principal_id
        LEFT JOIN oauth_clients oc
          ON oc.client_id = sa.client_id
        LEFT JOIN oauth_client_policies ocp
          ON ocp.client_id = oc.id
         AND ocp.scope_type = 'workspace'
         AND ocp.scope_id = sa.workspace_id
        WHERE sa.workspace_id = $1
        ORDER BY sa.created_at DESC
        "#,
    )
    .bind(access.workspace_id)
    .fetch_all(db)
    .await?;

    Ok(ServiceAccountsResult {
        service_accounts: rows
            .into_iter()
            .map(service_account_view_from_row)
            .collect(),
    })
}

pub async fn get_service_account(
    db: &PgPool,
    access: &WorkspaceAccess,
    service_account_id: Uuid,
) -> Result<ServiceAccountView, AppError> {
    let result = list_service_accounts(db, access).await?;
    result
        .service_accounts
        .into_iter()
        .find(|service_account| service_account.principal_id == service_account_id)
        .ok_or_else(|| {
            AppError::not_found(
                "service_account_not_found",
                "The requested service account was not found.",
            )
        })
}

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
    let now = now();

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

    sqlx::query(
        r#"
        INSERT INTO workspace_memberships (workspace_id, principal_id, role, status, source, created_at, updated_at)
        VALUES ($1, $2, $3::workspace_member_role, 'active', 'system', $4, $4)
        "#,
    )
    .bind(access.workspace_id)
    .bind(principal_id)
    .bind(role.to_string())
    .bind(now)
    .execute(&mut *tx)
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
        sqlx::query(
            r#"
            UPDATE workspace_memberships
            SET role = $3::workspace_member_role,
                updated_at = NOW()
            WHERE workspace_id = $1
              AND principal_id = $2
            "#,
        )
        .bind(access.workspace_id)
        .bind(service_account_id)
        .bind(role.to_string())
        .execute(&mut *tx)
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
