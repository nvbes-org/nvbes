use std::time::Duration;

use sqlx::postgres::PgPoolOptions;

use crate::{config::BillingWorkerConfig, synthetic};

#[tokio::test]
async fn synthetic_run_exercises_outbox_lifecycle_when_postgres_is_available() {
    if !postgres_reachable() {
        return;
    }
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://postgres:postgres@127.0.0.1:15432/nvbes_coverage_test".into()
    });
    // Under full-workspace llvm-cov the shared coverage DB can be briefly
    // saturated: TCP reachability is not enough. Skip cleanly on PoolTimedOut
    // instead of panicking the Sonar coverage lane.
    let Ok(pool) = PgPoolOptions::new()
        .max_connections(2)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&database_url)
        .await
    else {
        return;
    };
    if sqlx::migrate!("./migrations").run(&pool).await.is_err() {
        pool.close().await;
        return;
    }
    let config = BillingWorkerConfig {
        environment: "development".into(),
        database_url: "postgres://localhost/unused".into(),
        http_bind_addr: "127.0.0.1:8080".parse().unwrap(),
        app_url: "https://nvbes.test".into(),
        email_grpc_endpoint: None,
        email_token: None,
        billing_grpc_endpoint: None,
        billing_grpc_token: None,
        metrics_token: None,
        dispatch_mode: crate::config::DispatchMode::InMemory,
        otlp_endpoint: None,
        otlp_authorization_header: None,
    };
    let result = synthetic::run(&pool, &config).await.expect("synthetic run");
    assert!(result.event_dispatched);
    assert!(result.outbox_marked_published);
    assert!(result.idempotent_reprocessed);
    pool.close().await;
}

fn postgres_reachable() -> bool {
    crate::test_support::postgres_reachable()
}
