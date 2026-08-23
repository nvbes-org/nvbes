use crate::{
    grpc::pb::nvbes::developer::v1 as developer,
    test_support::{
        cleanup_tenant, has_developer_contract_schema, seed_developer_fixture, test_pool,
    },
};

#[tokio::test]
async fn activity_logs_preserve_risk_and_audit_feed_filters() {
    let pool = test_pool();
    if !has_activity_log_contract_schema(&pool).await {
        eprintln!("skipping test: local database is missing Developer activity log schema");
        return;
    }

    let fixture = seed_developer_fixture(&pool, "activity-logs").await;
    let client_id = format!("activity-{}", fixture.tenant_id.simple());
    let target_id = uuid::Uuid::new_v4();

    sqlx::query(
        r#"
        INSERT INTO risk_events (
          principal_id,
          event_type,
          risk_score,
          decision,
          metadata,
          created_at
        )
        VALUES ($1, 'login.failed', 0.9, 'challenge', jsonb_build_object('client_id', $2::text), $3)
        "#,
    )
    .bind(fixture.principal_id)
    .bind(&client_id)
    .bind(fixture.now)
    .execute(&pool)
    .await
    .expect("risk event should be seeded");

    sqlx::query(
        r#"
        INSERT INTO audit_events (
          tenant_id,
          actor_principal_id,
          action,
          target_type,
          target_id,
          metadata,
          event_hash,
          created_at
        )
        VALUES (
          $1,
          $2,
          'client.created',
          'oauth_client',
          $3,
          jsonb_build_object('client_id', $4::text),
          'seed',
          $5 + INTERVAL '1 second'
        )
        "#,
    )
    .bind(fixture.tenant_id)
    .bind(fixture.principal_id)
    .bind(target_id)
    .bind(&client_id)
    .bind(fixture.now)
    .execute(&pool)
    .await
    .expect("audit event should be seeded");

    let logs = super::list_activity_logs(
        &pool,
        developer::ListActivityLogsRequest {
            context: None,
            tenant_id: fixture.tenant_id.to_string(),
            user_id: String::new(),
            client_id: client_id.clone(),
            event_type: String::new(),
            limit: 100,
        },
    )
    .await
    .expect("activity logs should be listed");

    assert_eq!(logs.logs.len(), 2);
    assert!(
        logs.logs
            .iter()
            .all(|log| log.tenant_id == fixture.tenant_id.to_string())
    );
    assert!(logs.logs.iter().any(|log| log.event_type == "login.failed"));
    assert!(
        logs.logs
            .iter()
            .any(|log| log.event_type == "client.created")
    );

    let risk_logs = super::list_activity_logs(
        &pool,
        developer::ListActivityLogsRequest {
            context: None,
            tenant_id: fixture.tenant_id.to_string(),
            user_id: fixture.principal_id.to_string(),
            client_id,
            event_type: "login.failed".to_string(),
            limit: 100,
        },
    )
    .await
    .expect("activity logs should filter by event type");

    assert_eq!(risk_logs.logs.len(), 1);
    assert_eq!(risk_logs.logs[0].event_type, "login.failed");
    assert_eq!(risk_logs.logs[0].user_id, fixture.principal_id.to_string());

    cleanup_tenant(&pool, fixture.tenant_id).await;
}

async fn has_activity_log_contract_schema(pool: &sqlx::PgPool) -> bool {
    has_developer_contract_schema(pool).await
        && sqlx::query_scalar::<_, bool>(
            r#"
            SELECT
              to_regclass('public.risk_events') IS NOT NULL
              AND to_regclass('public.audit_events') IS NOT NULL
            "#,
        )
        .fetch_one(pool)
        .await
        .unwrap_or(false)
}
