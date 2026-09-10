use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use hmac::{Hmac, Mac};
use serde_json::json;
use sha2::Sha256;
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn failed_delivery_rolls_back_then_retries_once() {
    let pool = crate::database::connect(&std::env::var("DATABASE_URL").unwrap(), 3)
        .await
        .unwrap();
    crate::database::migrate(&pool).await.unwrap();
    let config = crate::config::BillingConfig {
        public_origin: None,
        account_authority: None,
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        database_url: String::new(),
        stripe_secret_key: "sk_test_dummy".into(),
        stripe_webhook_secret: "whsec_fixture".into(),
        stripe_api_base_url: "https://api.stripe.com".into(),
        identity_public_key_pem: None,
        identity_token_issuer: None,
        identity_token_key_id: None,
        identity_resource_client_id: None,
        identity_resource_secret: None,
        metrics_token: None,
        operator_token: Some("fixture".into()),
        app_url: "https://nvbes.test".into(),
    };
    let state = crate::app::BillingState {
        db: pool.clone(),
        tokens: crate::auth::TokenVerifier::new(&config).unwrap(),
        metrics: crate::metrics::install(),
        config,
    };
    let app = crate::app::create_router(state);
    let event_id = format!("evt_{}", Uuid::new_v4().simple());
    let plan = format!("retry_{}", Uuid::new_v4().simple());
    let account = Uuid::new_v4();
    let payload = json!({"id":event_id,"type":"customer.subscription.created","livemode":false,
    "created":chrono::Utc::now().timestamp(),"data":{"object":{
        "id":format!("sub_{}",Uuid::new_v4().simple()), "customer":"cus_fixture", "status":"active",
        "metadata":{"account_id":account,"plan_code":plan}
    }}})
    .to_string();
    let send = || {
        let timestamp = chrono::Utc::now().timestamp();
        let mut mac = Hmac::<Sha256>::new_from_slice(b"whsec_fixture").unwrap();
        mac.update(format!("{timestamp}.{payload}").as_bytes());
        let signature = mac
            .finalize()
            .into_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
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
    sqlx::query("INSERT INTO billing_plans(plan_code,name,stripe_price_id,currency,amount_cents) VALUES($1,'retry','price_fixture','eur',100)")
        .bind(&plan).execute(&pool).await.unwrap();
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
