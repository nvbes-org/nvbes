use chrono::Utc;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::{
    domains::auth::jwt::JwtService,
    domains::service_accounts::routes::tests::session::seed_admin_session,
    domains::service_accounts::routes::tests::state_support::test_database_url,
};

pub(super) struct ServiceAccountRouteFixture {
    pub(super) tenant_id: Uuid,
    pub(super) workspace_id: Uuid,
    pub(super) admin_principal_id: Uuid,
    pub(super) service_principal_id: Uuid,
    pub(super) client_id: String,
    pub(super) client_secret: String,
    pub(super) client_uuid: Uuid,
    pub(super) admin_token: String,
}

pub(super) async fn seed_admin_workspace(
    pool: &PgPool,
    redis: &nvbes_redis::RedisPool,
    jwt: &JwtService,
) -> ServiceAccountRouteFixture {
    crate::test_support::ensure_test_redis().await;
    crate::test_support::ensure_test_database(pool).await;
    let _ = test_database_url();

    let tenant_id = Uuid::new_v4();
    let workspace_id = Uuid::new_v4();
    let admin_principal_id = Uuid::new_v4();
    let service_principal_id = Uuid::new_v4();
    let client_uuid = Uuid::new_v4();
    let client_id = format!("sa_route_{}", Uuid::new_v4().simple());
    let client_secret = format!("sa_secret_{}", Uuid::new_v4().simple());
    let service_account_name = format!("Service Account {}", Uuid::new_v4().simple());
    let admin_email = format!("identity-admin-{}@example.com", Uuid::new_v4());
    let now = Utc::now();
    let plan_row = sqlx::query("SELECT id, code FROM plans ORDER BY created_at ASC LIMIT 1")
        .fetch_one(pool)
        .await
        .expect("plan should exist");
    let plan_id: Uuid = plan_row.get("id");
    let plan_code: String = plan_row.get("code");
    let password_hash =
        crate::domains::auth::hash_password("Admin$trongPassw0rd!").expect("password should hash");
    let client_secret_hash =
        crate::domains::oauth::hash_client_secret(&client_secret).expect("secret should hash");

    sqlx::query(
        r#"
        INSERT INTO tenants (id, kind, name, slug, status, security_tier, created_at, updated_at)
        VALUES ($1, 'team', 'Identity Route Test Tenant', $2, 'active', 'standard', $3, $3)
        "#,
    )
    .bind(tenant_id)
    .bind(format!("identity-route-test-{}", tenant_id))
    .bind(now)
    .execute(pool)
    .await
    .expect("tenant insert should succeed");

    sqlx::query(
        r#"
        INSERT INTO principals (id, tenant_id, principal_kind, status, display_name, created_at, updated_at)
        VALUES ($1, $2, 'human', 'active', 'Identity Admin', $3, $3)
        "#,
    )
    .bind(admin_principal_id)
    .bind(tenant_id)
    .bind(now)
    .execute(pool)
    .await
    .expect("admin principal insert should succeed");

    sqlx::query(
        r#"
        INSERT INTO users (
          principal_id, email, firstname, lastname, username, password_hash,
          email_verified_at, status, created_at, updated_at
        )
        VALUES ($1, $2, 'Identity', 'Admin', $3, $4, NOW(), 'active', $5, $5)
        "#,
    )
    .bind(admin_principal_id)
    .bind(&admin_email)
    .bind(format!("admin-{}", admin_principal_id))
    .bind(password_hash)
    .bind(now)
    .execute(pool)
    .await
    .expect("user insert should succeed");

    sqlx::query(
        r#"
        INSERT INTO workspaces (
          id,
          tenant_id,
          workspace_type,
          name,
          owner_user_id,
          plan_id,
          plan_code,
          created_at,
          updated_at
        )
        VALUES ($1, $2, 'team', 'Identity Route Test Workspace', $3, $4, $5, $6, $6)
        "#,
    )
    .bind(workspace_id)
    .bind(tenant_id)
    .bind(admin_principal_id)
    .bind(plan_id)
    .bind(&plan_code)
    .bind(now)
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

    sqlx::query(
        r#"
        INSERT INTO workspace_memberships (workspace_id, principal_id, role, status, source, created_at, updated_at)
        VALUES ($1, $2, 'owner', 'active', 'manual', $3, $3)
        "#,
    )
    .bind(workspace_id)
    .bind(admin_principal_id)
    .bind(now)
    .execute(pool)
    .await
    .expect("admin membership insert should succeed");

    sqlx::query(
        r#"
        INSERT INTO principals (id, tenant_id, principal_kind, status, display_name, created_at, updated_at)
        VALUES ($1, $2, 'service_account', 'active', $3, $4, $4)
        "#,
    )
    .bind(service_principal_id)
    .bind(tenant_id)
    .bind(&service_account_name)
    .bind(now)
    .execute(pool)
    .await
    .expect("service principal insert should succeed");

    sqlx::query(
        r#"
        INSERT INTO service_accounts (
          principal_id,
          tenant_id,
          workspace_id,
          created_by_principal_id,
          name,
          description,
          auth_method,
          client_id,
          last_rotated_at,
          created_at,
          updated_at
        )
        VALUES ($1, $2, $3, $4, $5, 'identity route test', 'oauth_client_credentials', $6, $7, $7, $7)
        "#,
    )
    .bind(service_principal_id)
    .bind(tenant_id)
    .bind(workspace_id)
    .bind(admin_principal_id)
    .bind(&service_account_name)
    .bind(&client_id)
    .bind(now)
    .execute(pool)
    .await
    .expect("service account insert should succeed");

    sqlx::query(
        r#"
        INSERT INTO workspace_memberships (workspace_id, principal_id, role, status, source, created_at, updated_at)
        VALUES ($1, $2, 'member', 'active', 'system', $3, $3)
        "#,
    )
    .bind(workspace_id)
    .bind(service_principal_id)
    .bind(now)
    .execute(pool)
    .await
    .expect("service membership insert should succeed");

    sqlx::query(
        r#"
        INSERT INTO oauth_clients (
          id,
          client_id,
          client_secret_hash,
          name,
          redirect_uris,
          tenant_id,
          owner_scope_type,
          owner_scope_id,
          client_type,
          revoked_at,
          created_at,
          updated_at
        )
        VALUES ($1, $2, $3, $4, ARRAY['https://example.com/callback'], $5, 'workspace', $6, 'service', NULL, $7, $7)
        "#,
    )
    .bind(client_uuid)
    .bind(&client_id)
    .bind(&client_secret_hash)
    .bind(format!("OAuth client for {service_account_name}"))
    .bind(tenant_id)
    .bind(workspace_id)
    .bind(now)
    .execute(pool)
    .await
    .expect("oauth client insert should succeed");

    sqlx::query(
        r#"
        INSERT INTO oauth_client_policies (
          client_id,
          scope_type,
          scope_id,
          allowed_scopes,
          allowed_audiences,
          allowed_resources,
          required_acr,
          status,
          created_at
        )
        VALUES ($1, 'workspace', $2, ARRAY['drive.files.read', 'drive.workspace.read'], ARRAY['nvbes-drive-api'], ARRAY['drive'], 'aal1', 'active', $3)
        "#,
    )
    .bind(client_uuid)
    .bind(workspace_id)
    .bind(now)
    .execute(pool)
    .await
    .expect("oauth policy insert should succeed");

    let mut fixture = ServiceAccountRouteFixture {
        tenant_id,
        workspace_id,
        admin_principal_id,
        service_principal_id,
        client_id,
        client_secret,
        client_uuid,
        admin_token: String::new(),
    };
    seed_admin_session(redis, jwt, &mut fixture, now).await;
    fixture
}
