use sqlx::PgPool;
use uuid::Uuid;

use crate::{config::BillingWorkerConfig, synthetic};

#[sqlx::test(migrations = "./migrations")]
async fn billing_worker_dispatches_outbox_and_runs_synthetic_lifecycle(pool: PgPool) {
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

    let result = synthetic::run(&pool, &config)
        .await
        .expect("synthetic worker run succeeds");

    assert!(result.event_dispatched);
    assert!(result.outbox_marked_published);
    assert!(result.idempotent_reprocessed);

    let published_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM billing_outbox WHERE event_uuid = $1 AND published_at IS NOT NULL",
    )
    .bind(result.outbox_event_uuid)
    .fetch_one(&pool)
    .await
    .expect("count query succeeds");

    assert_eq!(published_count, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn billing_worker_sweeps_unprocessed_outbox_events(pool: PgPool) {
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

    let state = crate::state::BillingWorkerState::new(config, pool.clone())
        .await
        .expect("state creation succeeds");

    for i in 0..3 {
        sqlx::query(
            "INSERT INTO billing_outbox (event_uuid, event_type, aggregate_id, payload) VALUES ($1, 'billing.invoice.paid.v1', $2, $3)",
        )
        .bind(Uuid::new_v4())
        .bind(Uuid::new_v4())
        .bind(serde_json::json!({ "invoice_id": format!("in_sweep_{i}") }))
        .execute(&pool)
        .await
        .expect("insert succeeds");
    }

    let swept = crate::dispatcher::sweep_pending(&state)
        .await
        .expect("sweep succeeds");
    assert!(swept >= 3);

    let pending_count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM billing_outbox WHERE published_at IS NULL")
            .fetch_one(&pool)
            .await
            .expect("count query succeeds");
    assert_eq!(pending_count, 0);
}
