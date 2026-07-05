use axum::http::StatusCode;
use sqlx::Row;
use uuid::Uuid;

use super::revoke_developer_secret_version;

#[tokio::test]
#[ignore = "requires a reachable PostgreSQL test database"]
async fn revoke_developer_secret_version_marks_secret_revoked_and_rejects_replay() {
    let _guard = crate::test_support::test_database_lock().lock().await;
    let pool = crate::test_support::isolated_test_pool(2);
    crate::test_support::ensure_test_database(&pool).await;

    let tenant_id = Uuid::new_v4();
    let client_uuid = Uuid::new_v4();
    let version_id = Uuid::new_v4();
    let client_id = format!("cli_{}", client_uuid.simple());
    let slug = format!("tenant-{}", tenant_id.simple());
    let mut tx = pool.begin().await.expect("transaction should start");

    sqlx::query("INSERT INTO tenants (id, name, slug) VALUES ($1, 'Developer tenant', $2)")
        .bind(tenant_id)
        .bind(slug)
        .execute(&mut *tx)
        .await
        .expect("tenant should be inserted");

    sqlx::query(
        r#"
        INSERT INTO oauth_clients (
          id, client_id, client_secret_hash, name, redirect_uris, tenant_id,
          owner_scope_type, owner_scope_id, client_type
        )
        VALUES ($1, $2, 'test-secret-hash', 'Developer OAuth Client',
          ARRAY['https://example.com/callback'], $3, 'tenant', $3, 'confidential')
        "#,
    )
    .bind(client_uuid)
    .bind(&client_id)
    .bind(tenant_id)
    .execute(&mut *tx)
    .await
    .expect("oauth client should be inserted");

    sqlx::query(
        r#"
        INSERT INTO developer_client_secret_versions (
          id, tenant_id, client_id, status, client_secret_hash, secret_last4
        )
        VALUES ($1, $2, $3, 'active', 'test-secret-hash', 'hash')
        "#,
    )
    .bind(version_id)
    .bind(tenant_id)
    .bind(&client_id)
    .execute(&mut *tx)
    .await
    .expect("developer secret version should be inserted");

    let revoked_client_id = revoke_developer_secret_version(&mut tx, tenant_id, version_id)
        .await
        .expect("secret version should be revoked");

    assert_eq!(revoked_client_id, client_id);

    let row = sqlx::query(
        r#"
        SELECT status::text AS status, revoked_at IS NOT NULL AS has_revoked_at
        FROM developer_client_secret_versions
        WHERE id = $1
        "#,
    )
    .bind(version_id)
    .fetch_one(&mut *tx)
    .await
    .expect("secret version should be readable");

    assert_eq!(row.get::<String, _>("status"), "revoked");
    assert!(row.get::<bool, _>("has_revoked_at"));

    let error = revoke_developer_secret_version(&mut tx, tenant_id, version_id)
        .await
        .expect_err("revoking the same version twice should fail");

    assert_eq!(error.status, StatusCode::NOT_FOUND);
    assert_eq!(error.code, "developer_secret_not_found");

    tx.rollback().await.expect("transaction should roll back");
}
