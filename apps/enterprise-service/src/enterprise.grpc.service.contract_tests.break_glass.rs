use super::*;

#[tokio::test]
async fn activation_and_revocation_round_trip() {
    let pool = test_pool();
    if !has_enterprise_contract_schema(&pool).await {
        eprintln!("skipping test: local database is missing Enterprise contract schema");
        return;
    }

    let fixture = seed_enterprise_fixture(&pool, "break-glass").await;
    let authentication = privileged_authentication_context(chrono::Utc::now().timestamp());
    break_glass::upsert_break_glass(
        &pool,
        fixture.tenant_id,
        fixture.principal_id,
        fixture.actor_id,
        "runbook-42",
        "customer incident",
        &authentication,
    )
    .await
    .expect("break-glass grant should be activated");

    let active = sqlx::query(
        r#"
        SELECT procedure_reference, reason, revoked_at
        FROM tenant_break_glass_accounts
        WHERE tenant_id = $1 AND principal_id = $2
        "#,
    )
    .bind(fixture.tenant_id)
    .bind(fixture.principal_id)
    .fetch_one(&pool)
    .await
    .expect("break-glass grant should be stored");
    assert_eq!(active.get::<String, _>("procedure_reference"), "runbook-42");
    assert_eq!(active.get::<String, _>("reason"), "customer incident");
    assert!(
        active
            .get::<Option<chrono::DateTime<chrono::Utc>>, _>("revoked_at")
            .is_none()
    );

    let listed =
        break_glass::list_break_glass_accounts(&pool, fixture.tenant_id, &[fixture.principal_id])
            .await
            .expect("active break-glass accounts should be readable");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].principal_id, fixture.principal_id.to_string());
    assert_eq!(listed[0].procedure_reference, "runbook-42");

    let changed = break_glass::revoke_break_glass(
        &pool,
        fixture.tenant_id,
        fixture.principal_id,
        fixture.actor_id,
        "incident closed",
    )
    .await
    .expect("break-glass grant should be revoked");
    assert_eq!(changed, 1);

    let revoked = sqlx::query(
        r#"
        SELECT revoked_reason, revoked_at
        FROM tenant_break_glass_accounts
        WHERE tenant_id = $1 AND principal_id = $2
        "#,
    )
    .bind(fixture.tenant_id)
    .bind(fixture.principal_id)
    .fetch_one(&pool)
    .await
    .expect("revoked break-glass grant should be stored");
    assert_eq!(
        revoked.get::<String, _>("revoked_reason"),
        "incident closed"
    );
    assert!(
        revoked
            .get::<Option<chrono::DateTime<chrono::Utc>>, _>("revoked_at")
            .is_some()
    );

    let listed_after_revoke =
        break_glass::list_break_glass_accounts(&pool, fixture.tenant_id, &[fixture.principal_id])
            .await
            .expect("revoked break-glass accounts should be filtered out");
    assert!(listed_after_revoke.is_empty());

    cleanup_tenant(&pool, fixture.tenant_id).await;
}
