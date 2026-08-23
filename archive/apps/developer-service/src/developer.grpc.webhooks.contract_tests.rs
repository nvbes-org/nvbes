use sqlx::Row;
use uuid::Uuid;

use crate::{
    grpc::pb::nvbes::developer::v1 as developer,
    test_support::{
        cleanup_tenant, has_developer_contract_schema, seed_developer_fixture, test_pool,
    },
};

#[tokio::test]
async fn webhook_creation_returns_signing_secret_and_scope_selection() {
    let pool = test_pool();
    if !has_developer_contract_schema(&pool).await {
        eprintln!("skipping test: local database is missing Developer contract schema");
        return;
    }

    let fixture = seed_developer_fixture(&pool, "webhook-create").await;
    let endpoint = super::create_webhook_endpoint(
        &pool,
        fixture.tenant_id,
        fixture.principal_id,
        developer::CreateWebhookEndpointRequest {
            context: None,
            tenant_id: fixture.tenant_id.to_string(),
            name: "Primary webhook".to_string(),
            url: "https://hooks.example.test/nvbes".to_string(),
            event_types: vec![
                "login.failed".to_string(),
                "client.created".to_string(),
                "login.failed".to_string(),
            ],
        },
    )
    .await
    .expect("webhook endpoint should be created");

    assert_eq!(endpoint.tenant_id, fixture.tenant_id.to_string());
    assert_eq!(endpoint.event_types, vec!["client.created", "login.failed"]);
    assert!(endpoint.signing_secret.starts_with("whsec_"));
    assert_eq!(
        endpoint.signing_secret_last4,
        endpoint
            .signing_secret
            .chars()
            .rev()
            .take(4)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<String>()
    );

    let listed = super::list_webhook_endpoints(&pool, fixture.tenant_id)
        .await
        .expect("webhook endpoints should be listed");
    let listed_endpoint = listed
        .endpoints
        .iter()
        .find(|candidate| candidate.endpoint_id == endpoint.endpoint_id)
        .expect("created endpoint should be listed");
    assert_eq!(listed_endpoint.event_types, endpoint.event_types);
    assert!(listed_endpoint.signing_secret.is_empty());

    cleanup_tenant(&pool, fixture.tenant_id).await;
}

#[tokio::test]
async fn api_log_reads_and_replay_use_webhook_deliveries() {
    let pool = test_pool();
    if !has_developer_contract_schema(&pool).await {
        eprintln!("skipping test: local database is missing Developer contract schema");
        return;
    }

    let fixture = seed_developer_fixture(&pool, "webhook-logs").await;
    let endpoint = super::create_webhook_endpoint(
        &pool,
        fixture.tenant_id,
        fixture.principal_id,
        developer::CreateWebhookEndpointRequest {
            context: None,
            tenant_id: fixture.tenant_id.to_string(),
            name: "Log webhook".to_string(),
            url: "https://hooks.example.test/logs".to_string(),
            event_types: vec!["session.revoked".to_string()],
        },
    )
    .await
    .expect("webhook endpoint should be created");
    let endpoint_id = Uuid::parse_str(&endpoint.endpoint_id).expect("endpoint id should be UUID");
    let delivery_id = Uuid::new_v4();
    let event_id = Uuid::new_v4();

    sqlx::query(
        r#"
        INSERT INTO developer_webhook_deliveries (
          id,
          endpoint_id,
          tenant_id,
          event_id,
          event_type,
          status,
          attempt_count,
          response_status,
          error_message,
          created_at
        )
        VALUES ($1, $2, $3, $4, 'session.revoked', 'failed', 2, 500, 'timeout', $5)
        "#,
    )
    .bind(delivery_id)
    .bind(endpoint_id)
    .bind(fixture.tenant_id)
    .bind(event_id)
    .bind(fixture.now)
    .execute(&pool)
    .await
    .expect("delivery should be seeded");

    let logs = super::list_api_logs(&pool, fixture.tenant_id)
        .await
        .expect("API logs should be listed");
    let api_log = logs
        .logs
        .iter()
        .find(|log| log.id == delivery_id.to_string())
        .expect("webhook delivery should be exposed as an API log");
    assert_eq!(api_log.source, "webhook");
    assert_eq!(api_log.severity, "error");
    assert_eq!(api_log.event_type, "session.revoked");

    let replayed = super::replay_webhook_delivery(&pool, fixture.tenant_id, delivery_id)
        .await
        .expect("failed delivery should be replayable");
    assert_eq!(replayed.status, "pending");
    assert_eq!(replayed.replayed_from_delivery_id, delivery_id.to_string());

    let replay_count = sqlx::query(
        r#"
        SELECT COUNT(*)::bigint AS replay_count
        FROM developer_webhook_deliveries
        WHERE tenant_id = $1
          AND replayed_from_delivery_id = $2
        "#,
    )
    .bind(fixture.tenant_id)
    .bind(delivery_id)
    .fetch_one(&pool)
    .await
    .expect("replay count should be readable")
    .get::<i64, _>("replay_count");
    assert_eq!(replay_count, 1);

    cleanup_tenant(&pool, fixture.tenant_id).await;
}
