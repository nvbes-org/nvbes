use std::time::Duration;

use chrono::{DateTime, Utc};
use sqlx::{PgPool, postgres::PgPoolOptions};
use uuid::Uuid;

use crate::grpc::pb::nvbes::platform::v1::{RequestContext, TenantContext};

pub struct DeveloperFixture {
    pub tenant_id: Uuid,
    pub principal_id: Uuid,
    pub now: DateTime<Utc>,
}

pub fn test_pool() -> PgPool {
    PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(1))
        .connect_lazy(&test_database_url_with_timeout())
        .expect("valid test database URL")
}

pub async fn has_developer_contract_schema(pool: &PgPool) -> bool {
    sqlx::query_scalar::<_, bool>(
        r#"
        SELECT
          to_regclass('public.tenants') IS NOT NULL
          AND to_regclass('public.principals') IS NOT NULL
          AND to_regclass('public.tenant_memberships') IS NOT NULL
          AND to_regclass('public.developer_scope_registry') IS NOT NULL
          AND to_regclass('public.developer_webhook_endpoints') IS NOT NULL
          AND to_regclass('public.developer_webhook_subscriptions') IS NOT NULL
          AND to_regclass('public.developer_webhook_deliveries') IS NOT NULL
          AND to_regclass('public.developer_sandbox_tenants') IS NOT NULL
          AND to_regclass('public.developer_token_debug_sessions') IS NOT NULL
          AND to_regclass('public.oauth_clients') IS NOT NULL
          AND to_regclass('public.oauth_client_policies') IS NOT NULL
          AND to_regclass('public.developer_client_secret_versions') IS NOT NULL
          AND EXISTS (
            SELECT 1
            FROM information_schema.columns
            WHERE table_schema = 'public'
              AND table_name = 'developer_webhook_deliveries'
              AND column_name = 'replayed_from_delivery_id'
          )
          AND EXISTS (
            SELECT 1
            FROM information_schema.columns
            WHERE table_schema = 'public'
              AND table_name = 'oauth_clients'
              AND column_name = 'requires_admin_consent'
          )
        "#,
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false)
}

pub async fn seed_developer_fixture(pool: &PgPool, label: &str) -> DeveloperFixture {
    let tenant_id = Uuid::new_v4();
    let principal_id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query(
        r#"
        INSERT INTO tenants (id, kind, name, slug, status, security_tier, created_at, updated_at)
        VALUES ($1, 'team', $2, $3, 'active', 'standard', $4, $4)
        "#,
    )
    .bind(tenant_id)
    .bind(format!("Developer contract {label}"))
    .bind(format!("developer-contract-{label}-{tenant_id}"))
    .bind(now)
    .execute(pool)
    .await
    .expect("tenant should be seeded");

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
    .bind(format!("Developer contract {label}"))
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

    DeveloperFixture {
        tenant_id,
        principal_id,
        now,
    }
}

pub async fn cleanup_tenant(pool: &PgPool, tenant_id: Uuid) {
    let sandbox_tenant_ids = sqlx::query_scalar::<_, Uuid>(
        "SELECT sandbox_tenant_id FROM developer_sandbox_tenants WHERE tenant_id = $1",
    )
    .bind(tenant_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let _ = sqlx::query("DELETE FROM tenants WHERE id = $1")
        .bind(tenant_id)
        .execute(pool)
        .await;

    for sandbox_tenant_id in sandbox_tenant_ids {
        let _ = sqlx::query("DELETE FROM tenants WHERE id = $1")
            .bind(sandbox_tenant_id)
            .execute(pool)
            .await;
    }
}

pub fn request_context(tenant_id: Uuid, principal_id: Uuid) -> RequestContext {
    let request_id = Uuid::new_v4().to_string();
    RequestContext {
        request_id: request_id.clone(),
        correlation_id: request_id,
        actor_principal_id: principal_id.to_string(),
        tenant: Some(TenantContext {
            tenant_id: tenant_id.to_string(),
            workspace_id: tenant_id.to_string(),
            region_id: "eu-fr".to_string(),
            data_residency: "eu".to_string(),
        }),
    }
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
