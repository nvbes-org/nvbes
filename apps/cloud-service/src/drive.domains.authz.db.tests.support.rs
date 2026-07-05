use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domains::auth::types::{AuthActorContext, AuthContext, AuthPrincipalKind};

pub(super) fn test_pool() -> PgPool {
    let url = std::env::var("DATABASE_URL")
        .or_else(|_| std::env::var("NVBES_DATABASE_URL"))
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/nvbes".to_string());
    PgPool::connect_lazy(&url).expect("valid pool")
}

pub(super) async fn cleanup(pool: &PgPool, key: &str) {
    sqlx::query("DELETE FROM users WHERE email = $1")
        .bind(format!("{key}@example.com"))
        .execute(pool)
        .await
        .ok();
}

pub(super) async fn db_supports_current_drive_schema(pool: &PgPool) -> bool {
    let users_have_id = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
          SELECT 1
          FROM information_schema.columns
          WHERE table_name = 'users'
            AND column_name = 'id'
        )
        "#,
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);

    let workspaces_have_tenant = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
          SELECT 1
          FROM information_schema.columns
          WHERE table_name = 'workspaces'
            AND column_name = 'tenant_id'
        )
        "#,
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);

    users_have_id && workspaces_have_tenant
}

pub(super) async fn seed_workspace(pool: &PgPool, key: &str) -> (Uuid, Uuid, Uuid) {
    let user_id = Uuid::new_v4();
    let workspace_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query(
        r#"
        INSERT INTO users (
          id,
          email,
          email_verified_at,
          display_name,
          status,
          mfa_enabled,
          created_at,
          updated_at
        )
        VALUES ($1, $2, NOW(), 'Drive Authz Test', 'active', false, $3, $3)
        "#,
    )
    .bind(user_id)
    .bind(format!("{key}@example.com"))
    .bind(now)
    .execute(pool)
    .await
    .expect("user insert should succeed");

    let plan_id: Uuid = sqlx::query_scalar("SELECT id FROM plans WHERE code = 'team_plus' LIMIT 1")
        .fetch_one(pool)
        .await
        .expect("plan should exist");

    sqlx::query(
        r#"
        INSERT INTO tenants (id, kind, name, slug, status, created_at, updated_at)
        VALUES ($1, 'team', 'Drive Tenant', $2, 'active', $3, $3)
        "#,
    )
    .bind(tenant_id)
    .bind(format!("drive-tenant-{tenant_id}"))
    .bind(now)
    .execute(pool)
    .await
    .expect("tenant insert should succeed");

    sqlx::query(
        r#"
        INSERT INTO workspaces (
          id,
          workspace_type,
          name,
          owner_user_id,
          owner_principal_id,
          plan_id,
          created_at,
          updated_at,
          tenant_id
        )
        VALUES ($1, 'team', 'Drive Workspace', $2, $3, $4, $5, $5, $6)
        "#,
    )
    .bind(workspace_id)
    .bind(user_id)
    .bind(user_id)
    .bind(plan_id)
    .bind(now)
    .bind(tenant_id)
    .execute(pool)
    .await
    .expect("workspace insert should succeed");

    sqlx::query(
        r#"
        INSERT INTO workspace_policies (
          workspace_id,
          member_can_create_share_links,
          require_admin_approval_for_member_share,
          default_share_link_ttl_days,
          max_share_link_ttl_days,
          updated_at
        )
        VALUES ($1, false, true, 7, 30, $2)
        ON CONFLICT (workspace_id) DO NOTHING
        "#,
    )
    .bind(workspace_id)
    .bind(now)
    .execute(pool)
    .await
    .expect("workspace policy insert should succeed");

    (user_id, workspace_id, tenant_id)
}

pub(super) fn machine_auth(
    principal_id: Uuid,
    tenant_id: Uuid,
    workspace_id: Uuid,
    role: &str,
) -> AuthContext {
    AuthContext {
        principal_id,
        principal_kind: AuthPrincipalKind::ServiceAccount,
        user_id: principal_id,
        email_verified_at: Some(Utc::now()),
        session_id: Uuid::nil(),
        tenant_id: Some(tenant_id),
        organization_id: None,
        workspace_id: Some(workspace_id),
        scope: "drive.files.read drive.workspace.read".to_string(),
        role: Some(role.to_string()),
        amr: vec!["m2m".to_string()],
        actor: None,
        acr: None,
        auth_time: Some(Utc::now()),
    }
}

pub(super) fn delegated_user_auth(
    user_id: Uuid,
    tenant_id: Uuid,
    workspace_id: Uuid,
    actor_role: &str,
) -> AuthContext {
    AuthContext {
        principal_id: user_id,
        principal_kind: AuthPrincipalKind::User,
        user_id,
        email_verified_at: Some(Utc::now()),
        session_id: Uuid::new_v4(),
        tenant_id: Some(tenant_id),
        organization_id: None,
        workspace_id: Some(workspace_id),
        scope: "drive.files.read drive.workspace.read".to_string(),
        role: Some("owner".to_string()),
        amr: vec!["pwd".to_string()],
        actor: Some(AuthActorContext {
            principal_id: Uuid::new_v4(),
            principal_kind: AuthPrincipalKind::ServiceAccount,
            tenant_id: Some(tenant_id),
            organization_id: None,
            workspace_id: Some(workspace_id),
            role: Some(actor_role.to_string()),
            client_id: Some("gxoc_actor".to_string()),
        }),
        acr: Some("aal1".to_string()),
        auth_time: Some(Utc::now()),
    }
}
