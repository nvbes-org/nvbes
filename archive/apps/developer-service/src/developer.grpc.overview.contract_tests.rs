use uuid::Uuid;

use crate::{
    grpc::pb::nvbes::developer::v1 as developer,
    test_support::{
        cleanup_tenant, has_developer_contract_schema, seed_developer_fixture, test_pool,
    },
};

#[tokio::test]
async fn overview_summary_counts_developer_owned_tables() {
    let pool = test_pool();
    if !has_developer_contract_schema(&pool).await {
        eprintln!("skipping test: local database is missing Developer contract schema");
        return;
    }

    let fixture = seed_developer_fixture(&pool, "overview").await;
    let oauth_client_id = Uuid::new_v4();
    let client_id = format!("overview-{}", fixture.tenant_id.simple());
    let webhook_endpoint_id = Uuid::new_v4();
    let sandbox_tenant_id = Uuid::new_v4();
    let scope_key = format!("contract.overview.{}", Uuid::new_v4().simple());

    sqlx::query(
        r#"
        INSERT INTO oauth_clients (
          id,
          client_id,
          client_secret_hash,
          name,
          redirect_uris,
          tenant_id,
          owner_scope_type,
          owner_scope_id,
          client_type,
          requires_admin_consent
        )
        VALUES ($1, $2, 'hash', 'Overview Client', '{}', $3, 'tenant', $3, 'confidential', false)
        "#,
    )
    .bind(oauth_client_id)
    .bind(&client_id)
    .bind(fixture.tenant_id)
    .execute(&pool)
    .await
    .expect("oauth client should be seeded");

    sqlx::query(
        "INSERT INTO developer_marketplace_apps (tenant_id, client_id, status, submitted_by) VALUES ($1, $2, 'pending', $3)",
    )
    .bind(fixture.tenant_id)
    .bind(&client_id)
    .bind(fixture.principal_id)
    .execute(&pool)
    .await
    .expect("marketplace app should be seeded");

    sqlx::query(
        "INSERT INTO developer_scope_registry (scope_key, display_name, description, risk, owner_team) VALUES ($1, 'Overview scope', 'Overview test scope', 'restricted', 'identity')",
    )
    .bind(&scope_key)
    .execute(&pool)
    .await
    .expect("scope should be seeded");

    sqlx::query(
        r#"
        INSERT INTO developer_webhook_endpoints (
          id,
          tenant_id,
          name,
          url,
          signing_secret_ciphertext,
          signing_secret_last4,
          created_by
        )
        VALUES ($1, $2, 'Overview webhook', 'https://example.test/webhook', 'secret', 'cret', $3)
        "#,
    )
    .bind(webhook_endpoint_id)
    .bind(fixture.tenant_id)
    .bind(fixture.principal_id)
    .execute(&pool)
    .await
    .expect("webhook endpoint should be seeded");

    sqlx::query(
        r#"
        INSERT INTO developer_webhook_deliveries (
          endpoint_id,
          tenant_id,
          event_id,
          event_type,
          status,
          attempt_count
        )
        VALUES ($1, $2, $3, 'user.created', 'failed', 1)
        "#,
    )
    .bind(webhook_endpoint_id)
    .bind(fixture.tenant_id)
    .bind(Uuid::new_v4())
    .execute(&pool)
    .await
    .expect("webhook delivery should be seeded");

    sqlx::query(
        "INSERT INTO developer_health_checks (tenant_id, target_type, target_id, check_kind, status, summary) VALUES ($1, 'webhook', $2, 'delivery', 'failing', 'failing')",
    )
    .bind(fixture.tenant_id)
    .bind(webhook_endpoint_id.to_string())
    .execute(&pool)
    .await
    .expect("health check should be seeded");

    sqlx::query(
        "INSERT INTO tenants (id, kind, name, slug, status, security_tier) VALUES ($1, 'team', 'Overview Sandbox', $2, 'active', 'sandbox')",
    )
    .bind(sandbox_tenant_id)
    .bind(format!("overview-sandbox-{sandbox_tenant_id}"))
    .execute(&pool)
    .await
    .expect("sandbox tenant should be seeded");

    sqlx::query(
        "INSERT INTO developer_sandbox_tenants (tenant_id, sandbox_tenant_id, status, data_profile) VALUES ($1, $2, 'active', 'minimal')",
    )
    .bind(fixture.tenant_id)
    .bind(sandbox_tenant_id)
    .execute(&pool)
    .await
    .expect("sandbox should be seeded");

    let overview = super::overview_summary(
        &pool,
        developer::GetOverviewSummaryRequest {
            context: None,
            tenant_id: fixture.tenant_id.to_string(),
        },
    )
    .await
    .expect("overview should be counted");

    assert_eq!(overview.tenant_id, fixture.tenant_id.to_string());
    assert_eq!(overview.oauth_clients, 1);
    assert_eq!(overview.marketplace_pending, 1);
    assert!(overview.high_risk_scopes >= 1);
    assert_eq!(overview.failed_webhook_deliveries, 1);
    assert_eq!(overview.unhealthy_integrations, 1);
    assert_eq!(overview.active_sandboxes, 1);

    let _ = sqlx::query("DELETE FROM developer_scope_registry WHERE scope_key = $1")
        .bind(&scope_key)
        .execute(&pool)
        .await;
    cleanup_tenant(&pool, fixture.tenant_id).await;
}
