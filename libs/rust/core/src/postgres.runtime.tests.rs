use super::{
    PostgresPoolSettings, connect_pool, connect_pool_with_url, connect_pool_with_url_and_role,
    health_check,
};
use crate::config::AppConfig;

#[tokio::test]
async fn connect_pool_with_url_establishes_working_pool() {
    let url = std::env::var("NVBES_SECURITY_TEST_DATABASE_URL")
        .expect("NVBES_SECURITY_TEST_DATABASE_URL is required");
    let pool = connect_pool_with_url(PostgresPoolSettings::from_max_connections(2), &url)
        .await
        .expect("pool should connect");
    health_check(&pool).await.expect("health check");
    pool.close().await;
}

#[tokio::test]
async fn connect_pool_uses_config_database_url() {
    let url = std::env::var("NVBES_SECURITY_TEST_DATABASE_URL")
        .expect("NVBES_SECURITY_TEST_DATABASE_URL is required");
    let config = AppConfig {
        database_url: url,
        database_max_connections: 2,
        ..AppConfig::default()
    };
    let pool = connect_pool(&config).await.expect("connect from config");
    health_check(&pool).await.expect("health check");
    pool.close().await;
}

#[tokio::test]
async fn connect_pool_with_postgres_role_sets_session_context() {
    let url = std::env::var("NVBES_SECURITY_TEST_DATABASE_URL")
        .expect("NVBES_SECURITY_TEST_DATABASE_URL is required");
    let pool = connect_pool_with_url_and_role(
        PostgresPoolSettings::from_max_connections(1),
        &url,
        "postgres",
    )
    .await
    .expect("postgres role should connect");
    health_check(&pool).await.expect("health check");
    pool.close().await;
}

#[tokio::test]
async fn role_connection_rejects_empty_role() {
    let url = std::env::var("NVBES_SECURITY_TEST_DATABASE_URL")
        .expect("NVBES_SECURITY_TEST_DATABASE_URL is required");
    let result =
        connect_pool_with_url_and_role(PostgresPoolSettings::from_max_connections(1), &url, "")
            .await;
    assert!(matches!(result, Err(sqlx::Error::Configuration(_))));
}
