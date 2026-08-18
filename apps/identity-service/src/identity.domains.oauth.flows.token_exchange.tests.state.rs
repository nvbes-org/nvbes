use crate::app::{AppConfig, AppState};
use base64::Engine;
use sqlx::PgPool;

pub(super) fn test_pool() -> PgPool {
    crate::test_support::shared_test_pool()
}

pub(super) async fn test_state(pool: &PgPool) -> AppState {
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

    let config = AppConfig {
        database_url: std::env::var("DATABASE_URL")
            .or_else(|_| std::env::var("NVBES_DATABASE_URL"))
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/nvbes".to_string()),
        environment: "development".to_string(),
        app_name: "identity-oauth-token-exchange-test".to_string(),
        otp_provider: "mock".to_string(),
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
    crate::test_support::cloud_mock::ensure_cloud_mock(pool);

    AppState::bootstrap(&config, pool.clone())
        .await
        .expect("app state bootstrap should succeed")
}

pub(super) async fn assert_current_oauth_schema(pool: &PgPool) {
    crate::test_support::ensure_test_database(pool).await;

    let client_assertion_supported = sqlx::query_scalar::<_, bool>(
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
    .expect("oauth client-assertion schema capability query should succeed");

    assert!(
        client_assertion_supported,
        "test database must include the current OAuth client assertion migrations"
    );
}

pub(super) fn basic_auth_header(client_id: &str, client_secret: &str) -> String {
    let encoded =
        base64::engine::general_purpose::STANDARD.encode(format!("{client_id}:{client_secret}"));
    format!("Basic {encoded}")
}
