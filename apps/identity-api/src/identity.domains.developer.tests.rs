use super::rbac::{DeveloperPermission, DeveloperRole, permissions_for_role};

#[test]
fn developer_admin_has_all_developer_permissions() {
    let permissions = permissions_for_role(DeveloperRole::DeveloperAdmin);

    assert!(permissions.contains(&DeveloperPermission::AppsCreate));
    assert!(permissions.contains(&DeveloperPermission::MarketplaceReview));
    assert!(permissions.contains(&DeveloperPermission::SecretsRotate));
    assert!(permissions.contains(&DeveloperPermission::WebhooksReplay));
    assert!(permissions.contains(&DeveloperPermission::LogsRead));
    assert!(permissions.contains(&DeveloperPermission::RbacManage));
}

#[test]
fn developer_integration_tester_can_use_tools_without_mutating_apps() {
    let permissions = permissions_for_role(DeveloperRole::IntegrationTester);

    assert!(permissions.contains(&DeveloperPermission::AppsRead));
    assert!(permissions.contains(&DeveloperPermission::TokensInspect));
    assert!(permissions.contains(&DeveloperPermission::HealthChecksRun));
    assert!(permissions.contains(&DeveloperPermission::SandboxUse));
    assert!(!permissions.contains(&DeveloperPermission::AppsCreate));
    assert!(!permissions.contains(&DeveloperPermission::SecretsRotate));
    assert!(!permissions.contains(&DeveloperPermission::WebhooksManage));
}

#[test]
fn developer_docs_viewer_has_only_docs_access() {
    let permissions = permissions_for_role(DeveloperRole::DocsViewer);

    assert_eq!(permissions, &[DeveloperPermission::DocsRead]);
}

#[test]
fn developer_role_db_literals_round_trip() {
    let cases = [
        (DeveloperRole::DeveloperAdmin, "developer_admin"),
        (DeveloperRole::AppManager, "app_manager"),
        (DeveloperRole::WebhookManager, "webhook_manager"),
        (DeveloperRole::LogViewer, "log_viewer"),
        (DeveloperRole::IntegrationTester, "integration_tester"),
        (DeveloperRole::DocsViewer, "docs_viewer"),
    ];

    for (role, literal) in cases {
        assert_eq!(role.as_db_str(), literal);
        assert_eq!(DeveloperRole::try_from(literal), Ok(role));
    }
}

#[test]
fn developer_role_rejects_invalid_db_literal() {
    let err = DeveloperRole::try_from("owner").expect_err("invalid literal should be rejected");

    assert_eq!(err.literal(), "owner");
}

#[tokio::test]
async fn test_marketplace_workflow() {
    let pool = crate::test_support::shared_test_pool();
    crate::test_support::ensure_test_database(&pool).await;

    let tenant_id = uuid::Uuid::new_v4();
    let principal_id = uuid::Uuid::new_v4();
    let now = chrono::Utc::now();

    sqlx::query(
        "INSERT INTO tenants (id, kind, name, slug, status, security_tier, created_at, updated_at)
         VALUES ($1, 'team', 'Dev Route Test Tenant', $2, 'active', 'standard', $3, $3)",
    )
    .bind(tenant_id)
    .bind(format!("dev-test-{}", tenant_id))
    .bind(now)
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO principals (id, tenant_id, principal_kind, status, display_name, created_at, updated_at)
         VALUES ($1, $2, 'human', 'active', 'Dev User', $3, $3)"
    )
    .bind(principal_id)
    .bind(tenant_id)
    .bind(now)
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO tenant_memberships (tenant_id, principal_id, principal_kind, role, status, created_at, updated_at)
         VALUES ($1, $2, 'human', 'member', 'active', $3, $3)"
    )
    .bind(tenant_id)
    .bind(principal_id)
    .bind(now)
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO developer_role_assignments (tenant_id, principal_id, role, created_at)
         VALUES ($1, $2, 'developer_admin', $3)",
    )
    .bind(tenant_id)
    .bind(principal_id)
    .bind(now)
    .execute(&pool)
    .await
    .unwrap();

    let client_uuid = uuid::Uuid::new_v4();
    let client_id = format!("client-{}", uuid::Uuid::new_v4().simple());
    let client_secret_hash =
        crate::domains::oauth::hash_client_secret("secret123").expect("hash secret");
    sqlx::query(
        "INSERT INTO oauth_clients (
          id, client_id, client_secret_hash, name, redirect_uris, tenant_id,
          owner_scope_type, owner_scope_id, client_type, created_at, updated_at
        )
        VALUES ($1, $2, $3, 'Test OAuth Client', '{}', $4, 'tenant', $4, 'confidential', $5, $5)",
    )
    .bind(client_uuid)
    .bind(&client_id)
    .bind(client_secret_hash)
    .bind(tenant_id)
    .bind(now)
    .execute(&pool)
    .await
    .unwrap();

    let auth = crate::http::middleware::jwt::AuthContext {
        user_id: principal_id,
        user_email: "dev@example.com".to_string(),
        display_name: "Dev User".to_string(),
        email_verified_at: None,
        mfa_enabled: false,
        tenant_id: Some(tenant_id),
        organization_id: None,
        workspace_id: None,
        workspace_region: None,
        token_type: "Bearer".to_string(),
        scope: String::new(),
        jti: "test-jti".to_string(),
        session_id: uuid::Uuid::new_v4(),
        acr: None,
        amr: vec![],
        auth_time: None,
        client_id: None,
        cnf_jkt: None,
    };

    let state = test_state(&pool).await;

    let list_response = super::routes::oauth::list_marketplace_apps(
        axum::extract::State(state.clone()),
        axum::Extension(auth.clone()),
    )
    .await
    .unwrap();
    assert_eq!(list_response.0.apps.len(), 0);

    let submit_response = super::routes::oauth::submit_marketplace_app(
        axum::extract::State(state.clone()),
        axum::Extension(auth.clone()),
        axum::extract::Path(client_id.clone()),
    )
    .await
    .unwrap();
    assert_eq!(submit_response.0.client_id, client_id);
    assert_eq!(submit_response.0.status, "pending");
    assert_eq!(submit_response.0.review_reason, None);

    let list_response2 = super::routes::oauth::list_marketplace_apps(
        axum::extract::State(state.clone()),
        axum::Extension(auth.clone()),
    )
    .await
    .unwrap();
    assert_eq!(list_response2.0.apps.len(), 1);
    assert_eq!(list_response2.0.apps[0].client_id, client_id);
    assert_eq!(list_response2.0.apps[0].status, "pending");

    let review_input = super::types::ReviewMarketplaceAppInput {
        status: "approved".to_string(),
        review_reason: Some("Looks good!".to_string()),
    };
    let review_response = super::routes::oauth::review_marketplace_app(
        axum::extract::State(state.clone()),
        axum::Extension(auth.clone()),
        axum::extract::Path(client_id.clone()),
        axum::Json(review_input),
    )
    .await
    .unwrap();
    assert_eq!(review_response.0.client_id, client_id);
    assert_eq!(review_response.0.status, "approved");
    assert_eq!(
        review_response.0.review_reason,
        Some("Looks good!".to_string())
    );

    let review_input2 = super::types::ReviewMarketplaceAppInput {
        status: "suspended".to_string(),
        review_reason: Some("Violated policy".to_string()),
    };
    let review_response2 = super::routes::oauth::review_marketplace_app(
        axum::extract::State(state.clone()),
        axum::Extension(auth.clone()),
        axum::extract::Path(client_id.clone()),
        axum::Json(review_input2),
    )
    .await
    .unwrap();
    assert_eq!(review_response2.0.client_id, client_id);
    assert_eq!(review_response2.0.status, "suspended");
    assert_eq!(
        review_response2.0.review_reason,
        Some("Violated policy".to_string())
    );
}

async fn test_state(pool: &sqlx::PgPool) -> crate::app::AppState {
    unsafe {
        std::env::set_var("NVBES_ENV", "development");
        std::env::set_var("NVBES_WORKSPACE_ROOT", "/Users/shayn/Development/nvbes");
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

#[tokio::test]
async fn test_scope_registry_crud() {
    let pool = crate::test_support::shared_test_pool();
    crate::test_support::ensure_test_database(&pool).await;

    let tenant_id = uuid::Uuid::new_v4();
    let principal_id = uuid::Uuid::new_v4();
    let now = chrono::Utc::now();

    sqlx::query(
        "INSERT INTO tenants (id, kind, name, slug, status, security_tier, created_at, updated_at)
         VALUES ($1, 'team', 'Scope Test Tenant', $2, 'active', 'standard', $3, $3)",
    )
    .bind(tenant_id)
    .bind(format!("scope-test-{}", tenant_id))
    .bind(now)
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO principals (id, tenant_id, principal_kind, status, display_name, created_at, updated_at)
         VALUES ($1, $2, 'human', 'active', 'Scope User', $3, $3)"
    )
    .bind(principal_id)
    .bind(tenant_id)
    .bind(now)
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO tenant_memberships (tenant_id, principal_id, principal_kind, role, status, created_at, updated_at)
         VALUES ($1, $2, 'human', 'member', 'active', $3, $3)"
    )
    .bind(tenant_id)
    .bind(principal_id)
    .bind(now)
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO developer_role_assignments (tenant_id, principal_id, role, created_at)
         VALUES ($1, $2, 'developer_admin', $3)",
    )
    .bind(tenant_id)
    .bind(principal_id)
    .bind(now)
    .execute(&pool)
    .await
    .unwrap();

    let auth = crate::http::middleware::jwt::AuthContext {
        user_id: principal_id,
        user_email: "scope-admin@example.com".to_string(),
        display_name: "Scope Admin User".to_string(),
        email_verified_at: None,
        mfa_enabled: false,
        tenant_id: Some(tenant_id),
        organization_id: None,
        workspace_id: None,
        workspace_region: None,
        token_type: "Bearer".to_string(),
        scope: String::new(),
        jti: "test-jti".to_string(),
        session_id: uuid::Uuid::new_v4(),
        acr: None,
        amr: vec![],
        auth_time: None,
        client_id: None,
        cnf_jkt: None,
    };

    let state = test_state(&pool).await;

    // 1. Create a scope
    let create_input = super::types::CreateScopeInput {
        scope_key: "test.scope.write".to_string(),
        display_name: "Test Scope Write".to_string(),
        description: "Allows writing test data".to_string(),
        risk: "medium".to_string(),
        owner_team: "Platform Security".to_string(),
        lifecycle: Some("active".to_string()),
        allowed_audiences: vec!["https://api.test.com".to_string()],
    };

    let create_response = super::routes::oauth::create_scope(
        axum::extract::State(state.clone()),
        axum::Extension(auth.clone()),
        axum::Json(create_input),
    )
    .await
    .unwrap();

    assert_eq!(create_response.0.scope_key, "test.scope.write");
    assert_eq!(create_response.0.display_name, "Test Scope Write");
    assert_eq!(create_response.0.risk, "medium");
    assert_eq!(create_response.0.lifecycle, "active");

    // Verify both tables have the scope
    let reg_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM developer_scope_registry WHERE scope_key = 'test.scope.write')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(reg_exists);

    let meta_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM oauth_scope_metadata WHERE scope = 'test.scope.write')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(meta_exists);

    // 2. List scopes and ensure ours is in the list
    let list_response = super::routes::oauth::list_scopes(
        axum::extract::State(state.clone()),
        axum::Extension(auth.clone()),
    )
    .await
    .unwrap();

    let found = list_response
        .0
        .scopes
        .iter()
        .any(|s| s.scope_key == "test.scope.write");
    assert!(found);

    // 3. Update the scope
    let update_input = super::types::UpdateScopeInput {
        display_name: "Test Scope Write Updated".to_string(),
        description: "Allows writing test data updated".to_string(),
        risk: "high".to_string(), // High risk requires admin consent
        owner_team: "Platform Security Team".to_string(),
        lifecycle: "active".to_string(),
        allowed_audiences: vec![
            "https://api.test.com".to_string(),
            "https://api2.test.com".to_string(),
        ],
    };

    let update_response = super::routes::oauth::update_scope(
        axum::extract::State(state.clone()),
        axum::Extension(auth.clone()),
        axum::extract::Path("test.scope.write".to_string()),
        axum::Json(update_input),
    )
    .await
    .unwrap();

    assert_eq!(update_response.0.display_name, "Test Scope Write Updated");
    assert_eq!(update_response.0.risk, "high");

    // Verify requires_admin_consent is updated to true in oauth_scope_metadata
    let requires_admin = sqlx::query_scalar::<_, bool>(
        "SELECT requires_admin_consent FROM oauth_scope_metadata WHERE scope = 'test.scope.write'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(requires_admin);

    // 4. Delete the scope
    super::routes::oauth::delete_scope(
        axum::extract::State(state.clone()),
        axum::Extension(auth.clone()),
        axum::extract::Path("test.scope.write".to_string()),
    )
    .await
    .unwrap();

    // Verify it is gone from both tables
    let reg_exists_after = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM developer_scope_registry WHERE scope_key = 'test.scope.write')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(!reg_exists_after);

    let meta_exists_after = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM oauth_scope_metadata WHERE scope = 'test.scope.write')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(!meta_exists_after);
}

#[tokio::test]
async fn test_webhook_replay_route() {
    let pool = crate::test_support::shared_test_pool();
    crate::test_support::ensure_test_database(&pool).await;

    let tenant_id = uuid::Uuid::new_v4();
    let principal_id = uuid::Uuid::new_v4();
    let now = chrono::Utc::now();

    sqlx::query(
        "INSERT INTO tenants (id, kind, name, slug, status, security_tier, created_at, updated_at)
         VALUES ($1, 'team', 'Webhooks Test Tenant', $2, 'active', 'standard', $3, $3)",
    )
    .bind(tenant_id)
    .bind(format!("webhooks-test-{}", tenant_id))
    .bind(now)
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO principals (id, tenant_id, principal_kind, status, display_name, created_at, updated_at)
         VALUES ($1, $2, 'human', 'active', 'Webhooks User', $3, $3)"
    )
    .bind(principal_id)
    .bind(tenant_id)
    .bind(now)
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO tenant_memberships (tenant_id, principal_id, principal_kind, role, status, created_at, updated_at)
         VALUES ($1, $2, 'human', 'member', 'active', $3, $3)"
    )
    .bind(tenant_id)
    .bind(principal_id)
    .bind(now)
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO developer_role_assignments (tenant_id, principal_id, role, created_at)
         VALUES ($1, $2, 'developer_admin', $3)",
    )
    .bind(tenant_id)
    .bind(principal_id)
    .bind(now)
    .execute(&pool)
    .await
    .unwrap();

    let endpoint_id = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO developer_webhook_endpoints (id, tenant_id, name, url, signing_secret_ciphertext, signing_secret_last4, created_by, created_at, updated_at)
         VALUES ($1, $2, 'Test Endpoint', 'https://example.com/webhook', 'secret', '1234', $3, $4, $4)"
    )
    .bind(endpoint_id)
    .bind(tenant_id)
    .bind(principal_id)
    .bind(now)
    .execute(&pool)
    .await
    .unwrap();

    let delivery_id = uuid::Uuid::new_v4();
    let event_id = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO developer_webhook_deliveries (id, endpoint_id, tenant_id, event_type, event_id, status, attempt_count, created_at)
         VALUES ($1, $2, $3, 'user.created', $4, 'failed', 1, $5)"
    )
    .bind(delivery_id)
    .bind(endpoint_id)
    .bind(tenant_id)
    .bind(event_id)
    .bind(now)
    .execute(&pool)
    .await
    .unwrap();

    let state = test_state(&pool).await;

    let auth = crate::http::middleware::jwt::AuthContext {
        user_id: principal_id,
        user_email: "dev@example.com".to_string(),
        display_name: "Dev User".to_string(),
        email_verified_at: None,
        mfa_enabled: false,
        tenant_id: Some(tenant_id),
        organization_id: None,
        workspace_id: None,
        workspace_region: None,
        token_type: "Bearer".to_string(),
        scope: String::new(),
        jti: "test-jti".to_string(),
        session_id: uuid::Uuid::new_v4(),
        acr: None,
        amr: vec![],
        auth_time: None,
        client_id: None,
        cnf_jkt: None,
    };

    let response = super::routes::webhooks::replay_delivery(
        axum::extract::State(state.clone()),
        axum::Extension(auth.clone()),
        axum::extract::Path(delivery_id),
    )
    .await
    .unwrap();

    assert_eq!(response.0.status, "pending");
    assert_eq!(response.0.endpoint_id, endpoint_id);
    assert_eq!(response.0.event_id, event_id);
    assert_eq!(response.0.replayed_from_delivery_id, Some(delivery_id));
}
