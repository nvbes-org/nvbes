use crate::{
    config::{BillingWorkerConfig, DispatchMode},
    state::BillingWorkerState,
};

fn base_config() -> BillingWorkerConfig {
    BillingWorkerConfig {
        environment: "development".into(),
        database_url: "postgres://localhost/unused".into(),
        http_bind_addr: "127.0.0.1:8080".parse().unwrap(),
        app_url: "http://localhost:3000".into(),
        email_grpc_endpoint: None,
        email_token: None,
        billing_grpc_endpoint: None,
        billing_grpc_token: None,
        metrics_token: None,
        dispatch_mode: DispatchMode::InMemory,
        otlp_endpoint: None,
        otlp_authorization_header: None,
    }
}

#[tokio::test]
async fn state_without_optional_clients_still_enqueues_locally() {
    let db = sqlx::postgres::PgPoolOptions::new()
        .min_connections(0)
        .max_connections(1)
        .connect_lazy("postgres://localhost/unused")
        .unwrap();
    let state = BillingWorkerState::new(base_config(), db).await.unwrap();
    assert!(state.email_client.is_none());
    assert!(state.billing_client.is_none());
    state.enqueue_event("evt_local").await.expect("enqueue");
    let mut rx = state
        .take_local_dispatch_receiver()
        .await
        .expect("local receiver");
    assert_eq!(rx.recv().await.as_deref(), Some("evt_local"));
    assert!(state.take_local_dispatch_receiver().await.is_none());
}

#[tokio::test]
async fn state_rejects_invalid_email_client_configuration() {
    let mut config = base_config();
    config.email_grpc_endpoint = Some("http://127.0.0.1:3041".into());
    config.email_token = Some("short".into());
    let db = sqlx::postgres::PgPoolOptions::new()
        .min_connections(0)
        .max_connections(1)
        .connect_lazy("postgres://localhost/unused")
        .unwrap();
    assert!(BillingWorkerState::new(config, db).await.is_err());
}

#[tokio::test]
async fn state_with_invalid_billing_endpoint_stays_optional() {
    let mut config = base_config();
    config.billing_grpc_endpoint = Some("not-a-uri".into());
    config.billing_grpc_token = Some("billing-internal-token-at-least-32-characters".into());
    let db = sqlx::postgres::PgPoolOptions::new()
        .min_connections(0)
        .max_connections(1)
        .connect_lazy("postgres://localhost/unused")
        .unwrap();
    let state = BillingWorkerState::new(config, db).await.unwrap();
    assert!(state.billing_client.is_none());
}
