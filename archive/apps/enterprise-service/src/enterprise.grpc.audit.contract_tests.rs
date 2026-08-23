use crate::{
    grpc::{audit, pb::nvbes::enterprise::v1 as enterprise},
    test_support::{
        cleanup_tenant, has_enterprise_contract_schema, seed_enterprise_fixture, test_pool,
    },
};

#[tokio::test]
async fn developer_secret_revocation_audit_is_recorded_by_enterprise() {
    let pool = test_pool();
    if !has_enterprise_contract_schema(&pool).await {
        eprintln!("skipping test: local database is missing Enterprise contract schema");
        return;
    }

    let fixture = seed_enterprise_fixture(&pool, "developer-secret-audit").await;
    let version_id = uuid::Uuid::new_v4();
    let response = audit::record_developer_secret_revoked(
        &pool,
        fixture.tenant_id,
        fixture.actor_id,
        enterprise::RecordDeveloperSecretRevokedRequest {
            context: None,
            tenant_id: fixture.tenant_id.to_string(),
            version_id: version_id.to_string(),
            client_id: "dev-client-123".to_string(),
        },
    )
    .await
    .expect("developer secret revocation audit should be recorded");

    assert_eq!(response.action, "enterprise.developer_secret.revoked");
    assert!(!response.event_id.is_empty());
    assert!(!response.created_at.is_empty());

    let audit_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM audit_events
        WHERE tenant_id = $1
          AND actor_principal_id = $2
          AND action = 'enterprise.developer_secret.revoked'
          AND target_type = 'developer_client_secret_version'
          AND target_id = $3
          AND metadata->>'client_id' = 'dev-client-123'
        "#,
    )
    .bind(fixture.tenant_id)
    .bind(fixture.actor_id)
    .bind(version_id)
    .fetch_one(&pool)
    .await
    .expect("audit event should be readable");
    assert_eq!(audit_count, 1);

    let listed = audit::list_audit_events(&pool, fixture.tenant_id, "tenant", "", 10)
        .await
        .expect("audit events should be listed");
    let event = listed
        .events
        .iter()
        .find(|event| event.event_id == response.event_id)
        .expect("recorded audit event should be in listing");
    assert_eq!(event.event_type, "enterprise.developer_secret.revoked");
    assert_eq!(event.actor_id, fixture.actor_id.to_string());
    assert_eq!(event.target_id, version_id.to_string());
    assert!(event.metadata_json.contains("dev-client-123"));

    cleanup_tenant(&pool, fixture.tenant_id).await;
}
