use axum::{Extension, extract::Path, extract::State};
use uuid::Uuid;

use super::support::{developer_auth, seed_developer_admin, test_state};

#[tokio::test]
async fn webhook_replay_route() {
    let pool = crate::test_support::shared_test_pool();
    crate::test_support::ensure_test_database(&pool).await;
    let fixture = seed_developer_admin(
        &pool,
        "Webhooks Test Tenant",
        "webhooks-test",
        "Webhooks User",
    )
    .await;

    let endpoint_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO developer_webhook_endpoints (id, tenant_id, name, url, signing_secret_ciphertext, signing_secret_last4, created_by, created_at, updated_at)
         VALUES ($1, $2, 'Test Endpoint', 'https://example.com/webhook', 'secret', '1234', $3, $4, $4)",
    )
    .bind(endpoint_id)
    .bind(fixture.tenant_id)
    .bind(fixture.principal_id)
    .bind(fixture.now)
    .execute(&pool)
    .await
    .expect("webhook endpoint should be seeded");

    let delivery_id = Uuid::new_v4();
    let event_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO developer_webhook_deliveries (id, endpoint_id, tenant_id, event_type, event_id, status, attempt_count, created_at)
         VALUES ($1, $2, $3, 'user.created', $4, 'failed', 1, $5)",
    )
    .bind(delivery_id)
    .bind(endpoint_id)
    .bind(fixture.tenant_id)
    .bind(event_id)
    .bind(fixture.now)
    .execute(&pool)
    .await
    .expect("webhook delivery should be seeded");

    let state = test_state(&pool).await;
    let auth = developer_auth(
        fixture.tenant_id,
        fixture.principal_id,
        "dev@example.com",
        "Dev User",
    );

    let response = crate::domains::developer::routes::webhooks::replay_delivery(
        State(state.clone()),
        Extension(auth),
        Path(delivery_id),
    )
    .await
    .expect("delivery should replay");

    assert_eq!(response.0.status, "pending");
    assert_eq!(response.0.endpoint_id, endpoint_id);
    assert_eq!(response.0.event_id, event_id);
    assert_eq!(response.0.replayed_from_delivery_id, Some(delivery_id));
}
