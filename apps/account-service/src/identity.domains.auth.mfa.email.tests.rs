use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use super::verified_primary_email_recipient;

async fn seed_principal(pool: &PgPool) -> (Uuid, Uuid) {
    crate::test_support::ensure_test_database(pool).await;

    let tenant_id = Uuid::new_v4();
    let principal_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO tenants (id, kind, name, slug, status, security_tier, created_at, updated_at)
        VALUES ($1, 'personal', 'Email MFA Test', $2, 'active', 'standard', NOW(), NOW())
        "#,
    )
    .bind(tenant_id)
    .bind(format!("email-mfa-{tenant_id}"))
    .execute(pool)
    .await
    .expect("tenant insert should succeed");

    sqlx::query(
        r#"
        INSERT INTO principals (id, tenant_id, principal_kind, status, display_name, created_at, updated_at)
        VALUES ($1, $2, 'human', 'active', 'Email MFA Test', NOW(), NOW())
        "#,
    )
    .bind(principal_id)
    .bind(tenant_id)
    .execute(pool)
    .await
    .expect("principal insert should succeed");

    (tenant_id, principal_id)
}

async fn insert_verified_email(pool: &PgPool, principal_id: Uuid, email: &str, is_primary: bool) {
    sqlx::query(
        r#"
        INSERT INTO user_email_addresses (
          principal_id, email, normalized_email, is_primary, verified_at, created_at, updated_at
        )
        VALUES ($1, $2, lower($2), $3, $4, $4, $4)
        "#,
    )
    .bind(principal_id)
    .bind(email)
    .bind(is_primary)
    .bind(Utc::now())
    .execute(pool)
    .await
    .expect("email address insert should succeed");
}

async fn cleanup_tenant(pool: &PgPool, tenant_id: Uuid) {
    sqlx::query("DELETE FROM tenants WHERE id = $1")
        .bind(tenant_id)
        .execute(pool)
        .await
        .ok();
}

#[tokio::test]
async fn login_code_recipient_is_only_the_verified_primary_email() {
    let pool = crate::test_support::shared_test_pool();
    let (tenant_id, principal_id) = seed_principal(&pool).await;
    let primary = format!("primary-{principal_id}@example.com");
    let secondary = format!("secondary-{principal_id}@example.com");
    insert_verified_email(&pool, principal_id, &secondary, false).await;
    insert_verified_email(&pool, principal_id, &primary, true).await;

    let recipient = verified_primary_email_recipient(&pool, principal_id)
        .await
        .expect("verified primary email should be selected");

    assert_eq!(recipient, primary);
    cleanup_tenant(&pool, tenant_id).await;
}

#[tokio::test]
async fn login_code_recipient_rejects_secondary_only_accounts() {
    let pool = crate::test_support::shared_test_pool();
    let (tenant_id, principal_id) = seed_principal(&pool).await;
    let secondary = format!("secondary-only-{principal_id}@example.com");
    insert_verified_email(&pool, principal_id, &secondary, false).await;

    let result = verified_primary_email_recipient(&pool, principal_id).await;

    assert!(result.is_err());
    cleanup_tenant(&pool, tenant_id).await;
}
