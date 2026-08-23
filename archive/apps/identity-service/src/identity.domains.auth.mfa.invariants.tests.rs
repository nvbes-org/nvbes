use uuid::Uuid;

use super::remove_factor;

#[tokio::test]
async fn active_last_factor_cannot_be_removed() {
    let db = crate::test_support::shared_test_pool();
    if !crate::test_support::is_test_database_available(&db).await {
        eprintln!("skipping test: database not available");
        return;
    }
    crate::test_support::ensure_test_database(&db).await;
    let _guard = crate::test_support::test_database_lock().lock().await;
    let (tenant_id, principal_id) = seed_user(&db).await;
    let factor_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO mfa_factors (id, principal_id, factor_type, status, label, confirmed_at)
        VALUES ($1, $2, 'totp', 'active', 'Only factor', NOW())
        "#,
    )
    .bind(factor_id)
    .bind(principal_id)
    .execute(&db)
    .await
    .expect("factor should be inserted");

    assert!(remove_factor(&db, principal_id, factor_id).await.is_err());
    let status: String = sqlx::query_scalar("SELECT status::text FROM mfa_factors WHERE id = $1")
        .bind(factor_id)
        .fetch_one(&db)
        .await
        .expect("factor should remain");
    assert_eq!(status, "active");
    cleanup(&db, tenant_id).await;
}

#[tokio::test]
async fn recovery_code_is_consumed_once_under_concurrency() {
    let db = crate::test_support::shared_test_pool();
    if !crate::test_support::is_test_database_available(&db).await {
        eprintln!("skipping test: database not available");
        return;
    }
    crate::test_support::ensure_test_database(&db).await;
    let _guard = crate::test_support::test_database_lock().lock().await;
    let (tenant_id, principal_id) = seed_user(&db).await;
    let code = "abcd-efgh-ijkl";
    sqlx::query(
        r#"
        INSERT INTO mfa_factors (
          id, principal_id, factor_type, status, label, factor_data, confirmed_at
        )
        VALUES ($1, $2, 'recovery_code', 'active', 'Recovery codes', $3, NOW())
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(principal_id)
    .bind(serde_json::json!({
        "codes": [crate::domains::auth::token_hash(code)]
    }))
    .execute(&db)
    .await
    .expect("recovery factor should be inserted");

    let (first, second) = tokio::join!(
        super::recovery::verify_recovery(&db, principal_id, code),
        super::recovery::verify_recovery(&db, principal_id, code),
    );
    assert_eq!((first.is_ok() as usize) + (second.is_ok() as usize), 1);
    cleanup(&db, tenant_id).await;
}

async fn seed_user(db: &sqlx::PgPool) -> (Uuid, Uuid) {
    let tenant_id = Uuid::new_v4();
    let principal_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO tenants (id, kind, name, slug, status, security_tier)
        VALUES ($1, 'personal', 'MFA invariant test', $2, 'active', 'standard')
        "#,
    )
    .bind(tenant_id)
    .bind(format!("mfa-invariant-{tenant_id}"))
    .execute(db)
    .await
    .expect("tenant should be inserted");
    sqlx::query(
        r#"
        INSERT INTO principals (id, tenant_id, principal_kind, status, display_name)
        VALUES ($1, $2, 'human', 'active', 'MFA invariant user')
        "#,
    )
    .bind(principal_id)
    .bind(tenant_id)
    .execute(db)
    .await
    .expect("principal should be inserted");
    sqlx::query(
        r#"
        INSERT INTO users (
          principal_id, email, password_hash, status
        )
        VALUES ($1, $2, 'test-password-hash', 'active')
        "#,
    )
    .bind(principal_id)
    .bind(format!("{principal_id}@example.test"))
    .execute(db)
    .await
    .expect("user should be inserted");
    (tenant_id, principal_id)
}

async fn cleanup(db: &sqlx::PgPool, tenant_id: Uuid) {
    sqlx::query("DELETE FROM tenants WHERE id = $1")
        .bind(tenant_id)
        .execute(db)
        .await
        .expect("tenant should be deleted");
}
