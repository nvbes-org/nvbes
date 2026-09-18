use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use axum::{Form, Json, Router, http::HeaderMap, routing::post};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{config::BillingConfig, synthetic};

#[sqlx::test(migrations = "./migrations")]
async fn billing_lifecycle_is_isolated_deduplicated_and_audited(pool: PgPool) {
    let workspace_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    let customer_id = format!("cus_test_{}", Uuid::new_v4().simple());
    let calls = Arc::new(AtomicUsize::new(0));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("local Stripe fixture binds");
    let stripe_api_base_url = format!("http://{}", listener.local_addr().unwrap());
    let fixture = Router::new().route(
        "/v1/customers",
        post({
            let calls = calls.clone();
            let customer_id = customer_id.clone();
            move |headers: HeaderMap, Form(form): Form<HashMap<String, String>>| {
                let calls = calls.clone();
                let customer_id = customer_id.clone();
                async move {
                    assert_eq!(headers["authorization"], "Bearer sk_test_mock_for_db_tests");
                    assert_eq!(
                        headers["idempotency-key"],
                        format!("customer_team_{workspace_id}")
                    );
                    assert_eq!(form["metadata[account_id]"], workspace_id.to_string());
                    assert_eq!(form["metadata[account_type]"], "team");
                    assert_eq!(form["email"], "billing-synthetic@nvbes.test");
                    calls.fetch_add(1, Ordering::SeqCst);
                    Json(json!({ "id": customer_id }))
                }
            }
        }),
    );
    let server = tokio::spawn(async move {
        axum::serve(listener, fixture)
            .await
            .expect("local Stripe fixture serves");
    });

    let config = BillingConfig {
        public_origin: None,
        browser_origins: Default::default(),
        account_authority: None,
        bind_addr: "127.0.0.1:8080".parse().unwrap(),
        database_url: "postgres://localhost/unused".into(),
        stripe_secret_key: "sk_test_mock_for_db_tests".into(),
        stripe_webhook_secret: "whsec_mock_secret".into(),
        stripe_api_base_url,
        identity_public_key_pem: None,
        identity_token_issuer: None,
        identity_token_key_id: None,
        identity_verification_keys: "[]".into(),
        identity_resource_client_id: None,
        identity_resource_secret: None,
        metrics_token: None,
        operator_token: Some("test_operator_token".into()),
        app_url: "https://nvbes.test".into(),
        email_grpc_endpoint: None,
        email_token: None,
    };

    let result = synthetic::run(&pool, &config, workspace_id, owner_id)
        .await
        .expect("synthetic Billing lifecycle succeeds");

    assert!(result.checkout_idempotent);
    assert!(result.webhook_deduplicated);
    assert!(result.out_of_order_protected);
    assert!(result.invoice_paid_processed);
    assert!(result.invoice_failed_processed);
    assert!(result.outbox_published);
    assert!(result.reconciliation_resolved);
    assert_eq!(result.subscription_status, "past_due");
    assert!(result.audit_events >= 2);
    assert!(result.outbox_events >= 3);
    assert_eq!(result.customer_id, customer_id);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    server.abort();
}
