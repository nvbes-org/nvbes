use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

pub struct DeveloperTestFixture {
    pub tenant_id: Uuid,
    pub principal_id: Uuid,
    pub now: DateTime<Utc>,
}

pub async fn seed_developer_admin(
    pool: &PgPool,
    tenant_label: &str,
    slug_prefix: &str,
    display_name: &str,
) -> DeveloperTestFixture {
    let tenant_id = Uuid::new_v4();
    let principal_id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query(
        "INSERT INTO tenants (id, kind, name, slug, status, security_tier, created_at, updated_at)
         VALUES ($1, 'team', $2, $3, 'active', 'standard', $4, $4)",
    )
    .bind(tenant_id)
    .bind(tenant_label)
    .bind(format!("{slug_prefix}-{tenant_id}"))
    .bind(now)
    .execute(pool)
    .await
    .expect("tenant should be seeded");

    sqlx::query(
        "INSERT INTO principals (id, tenant_id, principal_kind, status, display_name, created_at, updated_at)
         VALUES ($1, $2, 'human', 'active', $3, $4, $4)",
    )
    .bind(principal_id)
    .bind(tenant_id)
    .bind(display_name)
    .bind(now)
    .execute(pool)
    .await
    .expect("principal should be seeded");

    sqlx::query(
        "INSERT INTO tenant_memberships (tenant_id, principal_id, principal_kind, role, status, created_at, updated_at)
         VALUES ($1, $2, 'human', 'member', 'active', $3, $3)",
    )
    .bind(tenant_id)
    .bind(principal_id)
    .bind(now)
    .execute(pool)
    .await
    .expect("tenant membership should be seeded");

    sqlx::query(
        "INSERT INTO developer_role_assignments (tenant_id, principal_id, role, created_at)
         VALUES ($1, $2, 'developer_admin', $3)",
    )
    .bind(tenant_id)
    .bind(principal_id)
    .bind(now)
    .execute(pool)
    .await
    .expect("developer role should be seeded");

    DeveloperTestFixture {
        tenant_id,
        principal_id,
        now,
    }
}

pub fn developer_auth(
    tenant_id: Uuid,
    principal_id: Uuid,
    email: &str,
    display_name: &str,
) -> crate::http::middleware::jwt::AuthContext {
    crate::http::middleware::jwt::AuthContext {
        user_id: principal_id,
        user_email: email.to_string(),
        display_name: display_name.to_string(),
        email_verified_at: None,
        mfa_enabled: false,
        tenant_id: Some(tenant_id),
        organization_id: None,
        workspace_id: None,
        workspace_region: None,
        token_type: "Bearer".to_string(),
        scope: String::new(),
        jti: "test-jti".to_string(),
        session_id: Uuid::new_v4(),
        acr: None,
        amr: vec![],
        auth_time: None,
        client_id: None,
        cnf_jkt: None,
    }
}

pub async fn test_state(pool: &PgPool) -> crate::app::AppState {
    unsafe {
        std::env::set_var("NVBES_ENV", "development");
        std::env::set_var(
            "NVBES_WORKSPACE_ROOT",
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .ancestors()
                .nth(2)
                .expect("workspace root"),
        );
    }

    let config = crate::app::AppConfig {
        database_url: std::env::var("DATABASE_URL")
            .or_else(|_| std::env::var("NVBES_DATABASE_URL"))
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/nvbes".to_string()),
        environment: "development".to_string(),
        app_name: "identity-developer-route-test".to_string(),
        redis_url: std::env::var("NVBES_REDIS_URL")
            .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
        redis_password: std::env::var("NVBES_REDIS_PASSWORD")
            .ok()
            .filter(|value| !value.trim().is_empty()),
        redis_max_connections: std::env::var("NVBES_REDIS_MAX_CONNECTIONS")
            .ok()
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(10),
        ..Default::default()
    };

    crate::test_support::ensure_test_redis().await;
    crate::test_support::ensure_test_database(pool).await;

    crate::app::AppState::bootstrap(&config, pool.clone())
        .await
        .expect("app state bootstrap should succeed")
}
