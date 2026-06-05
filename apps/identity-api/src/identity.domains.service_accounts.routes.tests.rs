use axum::{
    extract::{Path, State},
    http::{HeaderMap, header::AUTHORIZATION},
};
use chrono::Utc;
use sqlx::{PgPool, Row};
use std::sync::OnceLock;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::{
    app::{AppConfig, AppState},
    domains::{
        auth::jwt::JwtService,
        auth::password::token_hash,
        auth::sessions::cache::{cached_session_from_login, current_session_ttl},
        oauth::flows::client_credentials_grant,
        oauth::service::ClientAuthentication,
        service_accounts::routes::{revoke_oauth_client, suspend_service_account},
    },
};

fn test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn test_database_url() -> String {
    std::env::var("DATABASE_URL")
        .or_else(|_| std::env::var("NVBES_DATABASE_URL"))
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/nvbes".to_string())
}

async fn test_state(pool: &PgPool) -> AppState {
    unsafe {
        std::env::set_var("NVBES_ENV", "development");
        std::env::set_var("NVBES_WORKSPACE_ROOT", "/Users/shayn/Development/nvbes");
    }

    let config = AppConfig {
        database_url: test_database_url(),
        environment: "development".to_string(),
        app_name: "identity-service-accounts-route-test".to_string(),
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

    AppState::bootstrap(&config, pool.clone())
        .await
        .expect("app state bootstrap should succeed")
}

async fn cleanup(
    pool: &PgPool,
    redis: &nvbes_redis::RedisPool,
    tenant_id: Uuid,
    admin_principal_id: Uuid,
    service_principal_id: Uuid,
    workspace_id: Uuid,
    client_uuid: Uuid,
) {
    let _ = nvbes_redis::session::clear_user_sessions(redis, &admin_principal_id.to_string()).await;
    sqlx::query("DELETE FROM oauth_client_policies WHERE client_id = $1")
        .bind(client_uuid)
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM oauth_clients WHERE id = $1")
        .bind(client_uuid)
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM service_accounts WHERE principal_id = $1")
        .bind(service_principal_id)
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM workspace_memberships WHERE principal_id = $1")
        .bind(service_principal_id)
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM workspace_memberships WHERE principal_id = $1")
        .bind(admin_principal_id)
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM users WHERE principal_id = $1")
        .bind(admin_principal_id)
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM principals WHERE id = $1")
        .bind(service_principal_id)
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM principals WHERE id = $1")
        .bind(admin_principal_id)
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM workspaces WHERE id = $1")
        .bind(workspace_id)
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM tenants WHERE id = $1")
        .bind(tenant_id)
        .execute(pool)
        .await
        .ok();
}

async fn seed_admin_workspace(
    pool: &PgPool,
    redis: &nvbes_redis::RedisPool,
    jwt: &JwtService,
) -> (Uuid, Uuid, Uuid, Uuid, String, String, Uuid, String) {
    crate::test_support::ensure_test_redis().await;
    crate::test_support::ensure_test_database(pool).await;

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

    let token_pair = jwt
        .generate_token_pair_with_session(
            admin_principal_id,
            Some(workspace_id),
            None,
            "drive.workspace.manage drive.files.read",
            Some(Uuid::new_v4()),
            Some(tenant_id),
            None,
            Some("aal2"),
            Some(vec!["pwd".to_string(), "otp".to_string()]),
            Some("identity-admin-client"),
            Some(now.timestamp()),
            None,
        )
        .expect("token pair should be created");

    let mut cached_session = cached_session_from_login(
        token_pair.session_id,
        admin_principal_id,
        Some(tenant_id),
        None,
        Some(workspace_id),
        None,
        None,
        token_hash(&token_pair.access_token),
        Some("aal2".to_string()),
        vec!["pwd".to_string(), "otp".to_string()],
        now,
        now,
        None,
        None,
        now + chrono::Duration::hours(2),
    );
    cached_session.step_up_verified_at = Some(now);
    cached_session.step_up_expires_at = Some(now + chrono::Duration::minutes(30));
    nvbes_redis::session::set_session(redis, &cached_session, current_session_ttl(&cached_session))
        .await
        .expect("redis session insert should succeed");

    (
        tenant_id,
        workspace_id,
        admin_principal_id,
        service_principal_id,
        client_id,
        client_secret,
        client_uuid,
        token_pair.access_token,
    )
}

fn bearer_headers(token: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        format!("Bearer {token}")
            .parse()
            .expect("authorization header should parse"),
    );
    headers
}

async fn assert_machine_token_fails(
    state: &AppState,
    client_id: &str,
    client_secret: &str,
    expected_code: &str,
) {
    let error = client_credentials_grant(
        state,
        ClientAuthentication {
            client_id: client_id.to_string(),
            client_secret: Some(client_secret.to_string()),
            client_assertion: None,
            client_assertion_verified: false,
        },
        Some("drive.files.read drive.workspace.read"),
        Some("nvbes-drive-api"),
    )
    .await
    .expect_err("machine token should be rejected");

    assert_eq!(error.code, expected_code);
}

#[tokio::test]
async fn suspend_service_account_endpoint_blocks_client_credentials_immediately() {
    let _guard = test_lock().lock().await;
    let pool = PgPool::connect_lazy(&test_database_url()).expect("valid pool");
    let state = test_state(&pool).await;
    let redis = state.redis.clone();
    let (
        _tenant_id,
        workspace_id,
        _admin_principal_id,
        service_principal_id,
        client_id,
        client_secret,
        _client_uuid,
        admin_token,
    ) = seed_admin_workspace(&pool, &redis, &state.jwt).await;

    let response = suspend_service_account(
        State(state.clone()),
        bearer_headers(&admin_token),
        Path((workspace_id, service_principal_id)),
    )
    .await
    .expect("service account suspension should succeed");

    assert_eq!(response.0.status, "suspended");
    assert_eq!(response.0.principal_id, service_principal_id);

    assert_machine_token_fails(
        &state,
        &client_id,
        &client_secret,
        "service_account_inactive",
    )
    .await;

    cleanup(
        &pool,
        &redis,
        _tenant_id,
        _admin_principal_id,
        service_principal_id,
        workspace_id,
        _client_uuid,
    )
    .await;
}

#[tokio::test]
async fn revoke_oauth_client_endpoint_detaches_client_and_blocks_client_credentials() {
    let _guard = test_lock().lock().await;
    let pool = PgPool::connect_lazy(&test_database_url()).expect("valid pool");
    let state = test_state(&pool).await;
    let redis = state.redis.clone();
    let (
        tenant_id,
        workspace_id,
        admin_principal_id,
        service_principal_id,
        client_id,
        client_secret,
        client_uuid,
        admin_token,
    ) = seed_admin_workspace(&pool, &redis, &state.jwt).await;

    let response = revoke_oauth_client(
        State(state.clone()),
        bearer_headers(&admin_token),
        Path((workspace_id, service_principal_id, client_id.clone())),
    )
    .await
    .expect("oauth client revocation should succeed");

    assert_eq!(response.0.principal_id, service_principal_id);
    assert!(response.0.oauth_clients.is_empty());

    assert_machine_token_fails(&state, &client_id, &client_secret, "invalid_client").await;

    cleanup(
        &pool,
        &redis,
        tenant_id,
        admin_principal_id,
        service_principal_id,
        workspace_id,
        client_uuid,
    )
    .await;
}

#[tokio::test]
async fn service_account_management_rejects_workspace_mismatch() {
    let _guard = test_lock().lock().await;
    let pool = PgPool::connect_lazy(&test_database_url()).expect("valid pool");
    let state = test_state(&pool).await;
    let redis = state.redis.clone();
    let (
        tenant_id,
        workspace_id,
        admin_principal_id,
        service_principal_id,
        _client_id,
        _client_secret,
        client_uuid,
        admin_token,
    ) = seed_admin_workspace(&pool, &redis, &state.jwt).await;
    let other_workspace_id = Uuid::new_v4();

    let error = suspend_service_account(
        State(state),
        bearer_headers(&admin_token),
        Path((other_workspace_id, service_principal_id)),
    )
    .await
    .expect_err("workspace mismatch should be rejected");

    assert_eq!(error.code, "workspace_context_mismatch");

    cleanup(
        &pool,
        &redis,
        tenant_id,
        admin_principal_id,
        service_principal_id,
        workspace_id,
        client_uuid,
    )
    .await;
}
