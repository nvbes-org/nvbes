use chrono::{DateTime, Utc};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::time::Duration;
use uuid::Uuid;

pub struct EnterpriseFixture {
    pub tenant_id: Uuid,
    pub principal_id: Uuid,
    pub actor_id: Uuid,
    pub now: DateTime<Utc>,
}

pub fn test_pool() -> PgPool {
    PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(1))
        .connect_lazy(&test_database_url_with_timeout())
        .expect("valid test database URL")
}

pub async fn has_enterprise_contract_schema(pool: &PgPool) -> bool {
    sqlx::query_scalar::<_, bool>(
        r#"
        SELECT
          to_regclass('public.tenants') IS NOT NULL
          AND to_regclass('public.principals') IS NOT NULL
          AND to_regclass('public.tenant_memberships') IS NOT NULL
          AND to_regclass('public.tenant_policies') IS NOT NULL
          AND to_regclass('public.tenant_break_glass_accounts') IS NOT NULL
          AND to_regclass('public.access_review_campaigns') IS NOT NULL
          AND to_regclass('public.access_review_items') IS NOT NULL
          AND to_regclass('public.access_review_schedules') IS NOT NULL
          AND to_regclass('public.federated_identity_providers') IS NOT NULL
          AND to_regclass('public.tenant_domains') IS NOT NULL
          AND to_regclass('public.scim_provisioning_connectors') IS NOT NULL
          AND to_regclass('public.workspace_memberships') IS NOT NULL
          AND to_regclass('public.workspace_invitations') IS NOT NULL
          AND to_regclass('public.mfa_factors') IS NOT NULL
          AND to_regclass('public.workspaces') IS NOT NULL
          AND to_regclass('public.audit_events') IS NOT NULL
          AND EXISTS (
            SELECT 1
            FROM information_schema.columns
            WHERE table_schema = 'public'
              AND table_name = 'tenant_policies'
              AND column_name = 'admin_session_ttl_hours'
          )
          AND EXISTS (
            SELECT 1
            FROM information_schema.columns
            WHERE table_schema = 'public'
              AND table_name = 'tenants'
              AND column_name = 'mfa_policy'
          )
          AND EXISTS (
            SELECT 1
            FROM information_schema.columns
            WHERE table_schema = 'public'
              AND table_name = 'workspaces'
              AND column_name = 'data_region'
          )
          AND EXISTS (
            SELECT 1
            FROM information_schema.columns
            WHERE table_schema = 'public'
              AND table_name = 'workspaces'
              AND column_name = 'jurisdiction'
          )
        "#,
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false)
}

pub async fn seed_enterprise_fixture(pool: &PgPool, label: &str) -> EnterpriseFixture {
    let tenant_id = Uuid::new_v4();
    let principal_id = Uuid::new_v4();
    let actor_id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query(
        r#"
        INSERT INTO tenants (id, kind, name, slug, status, security_tier, created_at, updated_at)
        VALUES ($1, 'enterprise', $2, $3, 'active', 'standard', $4, $4)
        "#,
    )
    .bind(tenant_id)
    .bind(format!("Enterprise contract {label}"))
    .bind(format!("enterprise-contract-{label}-{tenant_id}"))
    .bind(now)
    .execute(pool)
    .await
    .expect("tenant should be seeded");

    seed_principal(pool, tenant_id, principal_id, "Subject principal", now).await;
    seed_principal(pool, tenant_id, actor_id, "Actor principal", now).await;

    EnterpriseFixture {
        tenant_id,
        principal_id,
        actor_id,
        now,
    }
}

pub async fn membership_status(pool: &PgPool, tenant_id: Uuid, principal_id: Uuid) -> String {
    sqlx::query_scalar(
        "SELECT status::text FROM tenant_memberships WHERE tenant_id = $1 AND principal_id = $2",
    )
    .bind(tenant_id)
    .bind(principal_id)
    .fetch_one(pool)
    .await
    .expect("membership status should be readable")
}

pub async fn cleanup_tenant(pool: &PgPool, tenant_id: Uuid) {
    let _ = sqlx::query("DELETE FROM tenants WHERE id = $1")
        .bind(tenant_id)
        .execute(pool)
        .await;
}

async fn seed_principal(
    pool: &PgPool,
    tenant_id: Uuid,
    principal_id: Uuid,
    display_name: &str,
    now: DateTime<Utc>,
) {
    sqlx::query(
        r#"
        INSERT INTO principals (
          id,
          tenant_id,
          principal_kind,
          status,
          display_name,
          created_at,
          updated_at
        )
        VALUES ($1, $2, 'human', 'active', $3, $4, $4)
        "#,
    )
    .bind(principal_id)
    .bind(tenant_id)
    .bind(display_name)
    .bind(now)
    .execute(pool)
    .await
    .expect("principal should be seeded");

    sqlx::query(
        r#"
        INSERT INTO tenant_memberships (
          tenant_id,
          principal_id,
          principal_kind,
          role,
          status,
          created_at,
          updated_at
        )
        VALUES ($1, $2, 'human', 'member', 'active', $3, $3)
        "#,
    )
    .bind(tenant_id)
    .bind(principal_id)
    .bind(now)
    .execute(pool)
    .await
    .expect("tenant membership should be seeded");
}

fn test_database_url() -> String {
    std::env::var("DATABASE_URL")
        .or_else(|_| std::env::var("NVBES_DATABASE_URL"))
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/nvbes".to_string())
}

fn test_database_url_with_timeout() -> String {
    let url = test_database_url();
    if url.contains("connect_timeout=") {
        return url;
    }
    if url.contains('?') {
        format!("{url}&connect_timeout=1")
    } else {
        format!("{url}?connect_timeout=1")
    }
}
