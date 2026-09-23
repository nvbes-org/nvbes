use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use hmac::{Hmac, KeyInit, Mac};
use serde_json::json;
use sha2::Sha256;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

use crate::database::test_support::test_config;

fn sign(secret: &[u8], timestamp: i64, payload: &str) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret).expect("hmac key");
    mac.update(format!("{timestamp}.{payload}").as_bytes());
    mac.finalize()
        .into_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[sqlx::test(migrations = "./migrations")]
async fn failed_delivery_rolls_back_then_retries_once(pool: PgPool) {
    let mut config = test_config();
    config.stripe_webhook_secret = "whsec_fixture".into();
    let state = crate::app::BillingState {
        db: pool.clone(),
        tokens: crate::auth::TokenVerifier::new(&config).unwrap(),
        metrics: crate::metrics::install(),
        config,
        email_client: None,
    };
    let app = crate::app::create_router(state);
    let event_id = format!("evt_{}", Uuid::new_v4().simple());
    let plan = format!("retry_{}", Uuid::new_v4().simple());
    let account = Uuid::new_v4();
    let payload = json!({
        "id": event_id,
        "type": "customer.subscription.created",
        "livemode": false,
        "created": chrono::Utc::now().timestamp(),
        "data": {
            "object": {
                "id": format!("sub_{}", Uuid::new_v4().simple()),
                "customer": "cus_fixture",
                "status": "active",
                "metadata": { "account_id": account, "plan_code": plan }
            }
        }
    })
    .to_string();

    let send = || {
        let timestamp = chrono::Utc::now().timestamp();
        let signature = sign(b"whsec_fixture", timestamp, &payload);
        Request::post("/webhooks/stripe")
            .header("stripe-signature", format!("t={timestamp},v1={signature}"))
            .body(Body::from(payload.clone()))
            .unwrap()
    };

    assert_eq!(
        app.clone().oneshot(send()).await.unwrap().status(),
        StatusCode::INTERNAL_SERVER_ERROR
    );
    let effects: i64 =
        sqlx::query_scalar("SELECT count(*) FROM billing_outbox WHERE aggregate_id=$1")
            .bind(account)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(effects, 0);

    sqlx::query(
        "INSERT INTO billing_plans(plan_code,name,stripe_price_id,currency,amount_cents)
         VALUES($1,'retry','price_fixture','eur',100)",
    )
    .bind(&plan)
    .execute(&pool)
    .await
    .unwrap();

    assert_eq!(
        app.clone().oneshot(send()).await.unwrap().status(),
        StatusCode::OK
    );
    assert_eq!(app.oneshot(send()).await.unwrap().status(), StatusCode::OK);

    let effects: i64 =
        sqlx::query_scalar("SELECT count(*) FROM billing_outbox WHERE aggregate_id=$1")
            .bind(account)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(effects, 1);
    let status: String =
        sqlx::query_scalar("SELECT status FROM billing_webhook_events WHERE event_id=$1")
            .bind(event_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(status, "processed");
}

#[sqlx::test(migrations = "./migrations")]
async fn rejects_invalid_signature_and_livemode(pool: PgPool) {
    let config = test_config();
    let state = crate::app::BillingState {
        db: pool,
        tokens: crate::auth::TokenVerifier::new(&config).unwrap(),
        metrics: crate::metrics::install(),
        config,
        email_client: None,
    };
    let app = crate::app::create_router(state);
    let payload = json!({
        "id": "evt_live",
        "type": "invoice.paid",
        "livemode": true,
        "created": chrono::Utc::now().timestamp(),
        "data": { "object": { "id": "in_live" } }
    })
    .to_string();
    let timestamp = chrono::Utc::now().timestamp();
    let signature = sign(b"whsec_fixture", timestamp, &payload);
    let live = app
        .clone()
        .oneshot(
            Request::post("/webhooks/stripe")
                .header("stripe-signature", format!("t={timestamp},v1={signature}"))
                .body(Body::from(payload.clone()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(live.status(), StatusCode::BAD_REQUEST);

    let bad_sig = app
        .oneshot(
            Request::post("/webhooks/stripe")
                .header("stripe-signature", "t=0,v1=deadbeef")
                .body(Body::from(payload))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(bad_sig.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrations = "./migrations")]
async fn checkout_session_completed_updates_session_and_audit(pool: PgPool) {
    let config = test_config();
    let state = crate::app::BillingState {
        db: pool.clone(),
        tokens: crate::auth::TokenVerifier::new(&config).unwrap(),
        metrics: crate::metrics::install(),
        config,
        email_client: None,
    };
    let app = crate::app::create_router(state);

    let account = Uuid::new_v4();
    let session_id = format!("cs_{}", Uuid::new_v4().simple());
    sqlx::query(
        "INSERT INTO billing_checkout_sessions
         (idempotency_key, account_id, account_type, plan_code, stripe_session_id, stripe_customer_id, checkout_url)
         VALUES ($1, $2, 'team', 'standard_monthly', $3, 'cus_fixture', 'https://nvbes.test/checkout')",
    )
    .bind(format!("idem_{}", Uuid::new_v4()))
    .bind(account)
    .bind(&session_id)
    .execute(&pool)
    .await
    .unwrap();

    let event_id = format!("evt_{}", Uuid::new_v4().simple());
    let payload = json!({
        "id": event_id,
        "type": "checkout.session.completed",
        "livemode": false,
        "created": chrono::Utc::now().timestamp(),
        "data": { "object": { "id": session_id } }
    })
    .to_string();
    let timestamp = chrono::Utc::now().timestamp();
    let signature = sign(b"whsec_fixture", timestamp, &payload);
    let response = app
        .oneshot(
            Request::post("/webhooks/stripe")
                .header("stripe-signature", format!("t={timestamp},v1={signature}"))
                .body(Body::from(payload))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let status: String = sqlx::query_scalar(
        "SELECT status FROM billing_checkout_sessions WHERE stripe_session_id = $1",
    )
    .bind(&session_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(status, "completed");

    let audits: i64 =
        sqlx::query_scalar("SELECT count(*) FROM billing_audit_events WHERE account_id = $1")
            .bind(account)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(audits, 1);
}
