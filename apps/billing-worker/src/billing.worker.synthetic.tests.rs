use sqlx::PgPool;

use crate::{config::BillingWorkerConfig, synthetic};

#[tokio::test]
async fn synthetic_run_exercises_outbox_lifecycle_when_postgres_is_available() {
    if !postgres_reachable() {
        return;
    }
    let pool =
        PgPool::connect(&std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://postgres:postgres@127.0.0.1:5432/nvbes_billing".into()
        }))
        .await
        .expect("postgres");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrations");
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
    std::net::TcpStream::connect_timeout(
        &"127.0.0.1:5432".parse().unwrap(),
        std::time::Duration::from_millis(200),
    )
    .is_ok()
}
