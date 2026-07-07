use crate::{
    grpc::{admin_elevation, break_glass, pb::nvbes::enterprise::v1 as enterprise},
    test_support::{
        cleanup_tenant, has_enterprise_contract_schema, seed_enterprise_fixture, test_pool,
    },
};

#[tokio::test]
async fn admin_elevation_records_audit_and_break_glass_usage() {
    let pool = test_pool();
    if !has_enterprise_contract_schema(&pool).await {
        eprintln!("skipping test: local database is missing Enterprise contract schema");
        return;
    }

    let fixture = seed_enterprise_fixture(&pool, "admin-elevation-audit").await;
    break_glass::upsert_break_glass(
        &pool,
        fixture.tenant_id,
        fixture.actor_id,
        fixture.actor_id,
        "runbook-admin-elevation",
        "incident",
    )
    .await
    .expect("break-glass account should be seeded");

    let now = chrono::Utc::now();
    let response = admin_elevation::authorize_admin_elevation(
        &pool,
        fixture.tenant_id,
        fixture.actor_id,
        enterprise::AuthorizeAdminElevationRequest {
            context: None,
            tenant_id: fixture.tenant_id.to_string(),
            base_role: "owner".to_string(),
            duration_minutes: 15,
            step_up_expires_at: (now + chrono::Duration::minutes(10)).to_rfc3339(),
            session_expires_at: (now + chrono::Duration::minutes(30)).to_rfc3339(),
            break_glass: true,
            break_glass_reason: "incident".to_string(),
            break_glass_procedure_reference: "runbook-admin-elevation".to_string(),
        },
    )
    .await
    .expect("admin elevation should be authorized and recorded");

    assert_eq!(response.role, "admin");
    let audit_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM audit_events
        WHERE tenant_id = $1
          AND actor_principal_id = $2
          AND action IN (
            'enterprise.break_glass.used',
            'enterprise.admin_elevation.granted'
          )
        "#,
    )
    .bind(fixture.tenant_id)
    .bind(fixture.actor_id)
    .fetch_one(&pool)
    .await
    .expect("audit events should be readable");
    assert_eq!(audit_count, 2);

    let last_used_at = sqlx::query_scalar::<_, Option<chrono::DateTime<chrono::Utc>>>(
        r#"
        SELECT last_used_at
        FROM tenant_break_glass_accounts
        WHERE tenant_id = $1 AND principal_id = $2
        "#,
    )
    .bind(fixture.tenant_id)
    .bind(fixture.actor_id)
    .fetch_one(&pool)
    .await
    .expect("break-glass row should be readable");
    assert!(last_used_at.is_some());

    cleanup_tenant(&pool, fixture.tenant_id).await;
}
