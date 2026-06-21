use uuid::Uuid;

use super::{normalize_data_profile, reset_sandbox, upsert_sandbox};

#[test]
fn normalize_data_profile_defaults_to_minimal() {
    assert_eq!(
        normalize_data_profile(None).expect("none defaults"),
        "minimal"
    );
    assert_eq!(
        normalize_data_profile(Some(" ")).expect("blank defaults"),
        "minimal"
    );
}

#[test]
fn normalize_data_profile_trims_known_profiles() {
    assert_eq!(
        normalize_data_profile(Some(" oauth ")).expect("oauth profile"),
        "oauth"
    );
    assert_eq!(
        normalize_data_profile(Some("full")).expect("full profile"),
        "full"
    );
}

#[test]
fn normalize_data_profile_rejects_unknown_profiles() {
    let error = normalize_data_profile(Some("production")).expect_err("unknown profile is invalid");

    assert_eq!(error.code, "invalid_sandbox_data_profile");
}

#[tokio::test]
async fn reset_sandbox_provisions_full_profile_inside_sandbox_tenant() {
    let pool = crate::test_support::shared_test_pool();
    crate::test_support::ensure_test_database(&pool).await;

    let tenant_id = seed_client_tenant(&pool, "sandbox-full").await;

    let created = upsert_sandbox(&pool, tenant_id, Some("full"))
        .await
        .expect("sandbox should be created");
    assert_eq!(created.data_profile, "full");

    let reset = reset_sandbox(&pool, tenant_id)
        .await
        .expect("sandbox reset should succeed");
    assert_eq!(reset.status, "active");
    assert_eq!(reset.sandbox_tenant_id, created.sandbox_tenant_id);

    let tenant_security: (String, String) =
        sqlx::query_as("SELECT kind::text, security_tier FROM tenants WHERE id = $1")
            .bind(reset.sandbox_tenant_id)
            .fetch_one(&pool)
            .await
            .expect("sandbox tenant should exist");
    assert_eq!(tenant_security, ("team".to_string(), "sandbox".to_string()));

    let actor_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM principals
        WHERE tenant_id = $1
          AND principal_kind = 'human'
        "#,
    )
    .bind(reset.sandbox_tenant_id)
    .fetch_one(&pool)
    .await
    .expect("actor count should load");
    assert_eq!(actor_count, 1);

    let oauth_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM oauth_clients WHERE tenant_id = $1")
            .bind(reset.sandbox_tenant_id)
            .fetch_one(&pool)
            .await
            .expect("oauth count should load");
    assert_eq!(oauth_count, 1);

    let webhook_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM developer_webhook_endpoints WHERE tenant_id = $1")
            .bind(reset.sandbox_tenant_id)
            .fetch_one(&pool)
            .await
            .expect("webhook count should load");
    assert_eq!(webhook_count, 1);

    let client_tenant_oauth_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM oauth_clients WHERE tenant_id = $1")
            .bind(tenant_id)
            .fetch_one(&pool)
            .await
            .expect("client tenant oauth count should load");
    assert_eq!(client_tenant_oauth_count, 0);
}

async fn seed_client_tenant(pool: &sqlx::PgPool, slug_prefix: &str) -> Uuid {
    let tenant_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO tenants (id, kind, name, slug, status, security_tier)
        VALUES ($1, 'team', 'Sandbox Test Tenant', $2, 'active', 'standard')
        "#,
    )
    .bind(tenant_id)
    .bind(format!("{}-{}", slug_prefix, tenant_id.simple()))
    .execute(pool)
    .await
    .expect("client tenant should be seeded");

    tenant_id
}
