use axum::http::{HeaderMap, header::AUTHORIZATION};
use sqlx::PgPool;
use std::sync::OnceLock;
use tokio::sync::Mutex;

use crate::{
    app::{AppConfig, AppState},
    domains::{
        oauth::flows::client_credentials_grant, oauth::service::ClientAuthentication,
        service_accounts::routes::tests::seed::ServiceAccountRouteFixture,
    },
};

pub(super) fn test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

pub(super) fn test_database_url() -> String {
    std::env::var("DATABASE_URL")
        .or_else(|_| std::env::var("NVBES_DATABASE_URL"))
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/nvbes".to_string())
}

pub(super) async fn test_state(pool: &PgPool) -> AppState {
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

pub(super) fn bearer_headers(token: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        format!("Bearer {token}")
            .parse()
            .expect("authorization header should parse"),
    );
    headers
}

pub(super) async fn assert_machine_token_fails(
    state: &AppState,
    fixture: &ServiceAccountRouteFixture,
    expected_code: &str,
) {
    let error = client_credentials_grant(
        &state.db,
        &state.jwt,
        ClientAuthentication {
            client_id: fixture.client_id.clone(),
            client_secret: Some(fixture.client_secret.clone()),
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
