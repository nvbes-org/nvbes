use chrono::Utc;
use uuid::Uuid;

use super::soft_delete_personal_account_tx;

#[tokio::test]
async fn personal_account_deletion_preserves_tenant() {
    let db = crate::test_support::shared_test_pool();
    crate::test_support::ensure_test_database(&db).await;
    let _guard = crate::test_support::test_database_lock().lock().await;

    let tenant_id = Uuid::new_v4();
    let principal_id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query(
        r#"
        INSERT INTO tenants (id, kind, name, slug, status, security_tier, created_at, updated_at)
        VALUES ($1, 'personal', 'Account deletion tenant', $2, 'active', 'standard', $3, $3)
        "#,
    )
    .bind(tenant_id)
    .bind(format!("account-deletion-{tenant_id}"))
    .bind(now)
    .execute(&db)
    .await
    .expect("tenant should be inserted");

    sqlx::query(
        r#"
        INSERT INTO principals (
          id, tenant_id, principal_kind, status, display_name, created_at, updated_at
        )
        VALUES ($1, $2, 'human', 'active', 'Account deletion user', $3, $3)
        "#,
    )
    .bind(principal_id)
    .bind(tenant_id)
    .bind(now)
    .execute(&db)
    .await
    .expect("principal should be inserted");

    sqlx::query(
        r#"
        INSERT INTO users (
          principal_id, email, firstname, lastname, username, password_hash,
          email_verified_at, status, created_at, updated_at
        )
        VALUES ($1, $2, 'Account', 'Deletion', $3, 'test-password-hash', $4, 'active', $4, $4)
        "#,
    )
    .bind(principal_id)
    .bind(format!("{principal_id}@example.test"))
    .bind(format!("account-deletion-{principal_id}"))
    .bind(now)
    .execute(&db)
    .await
    .expect("user should be inserted");

    let mut tx = db.begin().await.expect("transaction should start");
    soft_delete_personal_account_tx(&mut tx, principal_id)
        .await
        .expect("personal account deletion should succeed");
    tx.commit().await.expect("transaction should commit");

    let user_status: String =
        sqlx::query_scalar("SELECT status::text FROM users WHERE principal_id = $1")
            .bind(principal_id)
            .fetch_one(&db)
            .await
            .expect("user status should be readable");
    let principal_status: String =
        sqlx::query_scalar("SELECT status::text FROM principals WHERE id = $1")
            .bind(principal_id)
            .fetch_one(&db)
            .await
            .expect("principal status should be readable");
    let tenant_status: String =
        sqlx::query_scalar("SELECT status::text FROM tenants WHERE id = $1")
            .bind(tenant_id)
            .fetch_one(&db)
            .await
            .expect("tenant status should be readable");

    assert_eq!(user_status, "deleted");
    assert_eq!(principal_status, "deleted");
    assert_eq!(tenant_status, "active");

    sqlx::query("DELETE FROM tenants WHERE id = $1")
        .bind(tenant_id)
        .execute(&db)
        .await
        .expect("test tenant should be deleted");
}
