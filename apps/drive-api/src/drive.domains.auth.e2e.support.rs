use axum::Router;
use chrono::Utc;
use nvbes_core::config::AppConfig;
use sqlx::postgres::PgPool;
use std::sync::OnceLock;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use uuid::Uuid;

pub(super) fn test_database_url() -> String {
    std::env::var("DATABASE_URL")
        .or_else(|_| std::env::var("NVBES_DATABASE_URL"))
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/nvbes".to_string())
}

pub(super) fn test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

pub(super) async fn spawn_identity_server(state: nvbes_identity_api::app::AppState) -> String {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("test server should bind");
    let addr = listener.local_addr().expect("listener addr");
    let base_url = format!("http://{addr}");
    let router: Router = nvbes_identity_api::app::build_router(state);
    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("test server should run");
    });
    base_url
}

pub(super) async fn identity_state(pool: &PgPool) -> nvbes_identity_api::app::AppState {
    unsafe {
        std::env::set_var("NVBES_ENV", "development");
        std::env::set_var("NVBES_WORKSPACE_ROOT", "/Users/shayn/Development/nvbes");
    }

    let config = nvbes_identity_api::app::AppConfig {
        database_url: test_database_url(),
        environment: "development".to_string(),
        app_name: "drive-auth-e2e-identity-test".to_string(),
        ..Default::default()
    };

    nvbes_identity_api::app::AppState::bootstrap(&config, pool.clone())
        .await
        .expect("identity app state bootstrap should succeed")
}

pub(super) async fn drive_app() -> axum::Router {
    unsafe {
        std::env::set_var("NVBES_ENV", "development");
        std::env::set_var("NVBES_WORKSPACE_ROOT", "/Users/shayn/Development/nvbes");
    }

    let config = AppConfig {
        database_url: test_database_url(),
        environment: "development".to_string(),
        app_name: "drive-auth-e2e-drive-test".to_string(),
        ..Default::default()
    };
    let db = crate::db::Database::connect(&config)
        .await
        .expect("drive database should connect");
    crate::app::build_app(config, db)
        .await
        .expect("drive app should build")
}

pub(super) async fn db_supports_current_schema(pool: &PgPool) -> bool {
    let identity_oauth = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
          SELECT 1
          FROM information_schema.columns
          WHERE table_name = 'oauth_clients'
            AND column_name = 'client_assertion_required'
        )
        "#,
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);

    let drive_workspace = sqlx::query_scalar::<_, bool>(
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

    let drive_users_have_id = sqlx::query_scalar::<_, bool>(
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

    let drive_created_by_principal = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
          SELECT 1
          FROM information_schema.columns
          WHERE table_name = 'storage_objects'
            AND column_name = 'created_by_principal_id'
        )
        "#,
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);

    let drive_share_links_created_by_principal = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
          SELECT 1
          FROM information_schema.columns
          WHERE table_name = 'share_links'
            AND column_name = 'created_by_principal_id'
        )
        "#,
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);

    let drive_upload_sessions_created_by_principal = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
          SELECT 1
          FROM information_schema.columns
          WHERE table_name = 'upload_sessions'
            AND column_name = 'created_by_principal_id'
        )
        "#,
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);

    identity_oauth
        && drive_workspace
        && drive_users_have_id
        && drive_created_by_principal
        && drive_share_links_created_by_principal
        && drive_upload_sessions_created_by_principal
}

pub(super) async fn cleanup(pool: &PgPool, tenant_id: Uuid, owner_email: &str) {
    sqlx::query("DELETE FROM users WHERE email = $1")
        .bind(owner_email)
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM tenants WHERE id = $1")
        .bind(tenant_id)
        .execute(pool)
        .await
        .ok();
}

pub(super) async fn issue_machine_token(
    base_url: &str,
    client_id: &str,
    client_secret: &str,
) -> String {
    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("http client should build");
    let token_response = http
        .post(format!("{base_url}/oauth/token"))
        .basic_auth(client_id, Some(client_secret))
        .json(&serde_json::json!({
            "grant_type": "client_credentials",
            "scope": "drive.files.read drive.workspace.read",
            "audience": "nvbes-drive-api",
        }))
        .send()
        .await
        .expect("token endpoint should respond");

    assert!(token_response.status().is_success());
    let token_body = token_response
        .json::<serde_json::Value>()
        .await
        .expect("token body should be valid json");
    token_body["access_token"]
        .as_str()
        .expect("access token should exist")
        .to_string()
}

pub(super) async fn seed_machine_workspace_context(
    pool: &PgPool,
) -> (Uuid, Uuid, String, String, Uuid, String) {
    let tenant_id = Uuid::new_v4();
    let workspace_id = Uuid::new_v4();
    let owner_user_id = Uuid::new_v4();
    let principal_id = Uuid::new_v4();
    let client_uuid = Uuid::new_v4();
    let client_id = format!("gxoc_drive_e2e_{}", Uuid::new_v4().simple());
    let client_secret = format!("gxo_drive_e2e_{}", Uuid::new_v4().simple());
    let client_secret_hash = nvbes_identity_api::domains::oauth::hash_client_secret(&client_secret)
        .expect("secret should hash");
    let owner_email = format!("drive-auth-e2e-{}@example.com", Uuid::new_v4());
    let now = Utc::now();

    let plan_id: Uuid = sqlx::query_scalar("SELECT id FROM plans ORDER BY created_at ASC LIMIT 1")
        .fetch_one(pool)
        .await
        .expect("plan should exist");

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
        VALUES ($1, $2, NOW(), 'Drive E2E Owner', 'active', false, $3, $3)
        "#,
    )
    .bind(owner_user_id)
    .bind(&owner_email)
    .bind(now)
    .execute(pool)
    .await
    .expect("owner user insert should succeed");

    sqlx::query(
        r#"
        INSERT INTO tenants (id, kind, name, slug, status, security_tier, created_at, updated_at)
        VALUES ($1, 'team', 'Drive E2E Tenant', $2, 'active', 'standard', $3, $3)
        "#,
    )
    .bind(tenant_id)
    .bind(format!("drive-auth-e2e-{tenant_id}"))
    .bind(now)
    .execute(pool)
    .await
    .expect("tenant insert should succeed");

    sqlx::query(
        r#"
        INSERT INTO workspaces (
          id,
          tenant_id,
          workspace_type,
          name,
          owner_user_id,
          owner_principal_id,
          plan_id,
          plan_code,
          created_at,
          updated_at
        )
        VALUES ($1, $2, 'team', 'Drive E2E Workspace', $3, $3, $4, 'team_plus', $5, $5)
        "#,
    )
    .bind(workspace_id)
    .bind(tenant_id)
    .bind(owner_user_id)
    .bind(plan_id)
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
        INSERT INTO principals (id, tenant_id, principal_kind, status, display_name, created_at, updated_at)
        VALUES ($1, $2, 'service_account', 'active', 'Drive E2E Robot', $3, $3)
        "#,
    )
    .bind(principal_id)
    .bind(tenant_id)
    .bind(now)
    .execute(pool)
    .await
    .expect("principal insert should succeed");

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
        VALUES ($1, $2, $3, NULL, 'Drive E2E Robot', 'drive e2e machine', 'oauth_client_credentials', $4, $5, $5, $5)
        "#,
    )
    .bind(principal_id)
    .bind(tenant_id)
    .bind(workspace_id)
    .bind(&client_id)
    .bind(now)
    .execute(pool)
    .await
    .expect("service account insert should succeed");

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
        VALUES ($1, $2, $3, 'Drive E2E Client', ARRAY['https://example.com/callback'], $4, 'workspace', $5, 'service', NULL, $6, $6)
        "#,
    )
    .bind(client_uuid)
    .bind(&client_id)
    .bind(&client_secret_hash)
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

    (
        tenant_id,
        workspace_id,
        client_id,
        client_secret,
        principal_id,
        owner_email,
    )
}
