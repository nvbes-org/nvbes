use super::*;
use chrono::Duration;
use sqlx::Row;

#[tokio::test]
#[ignore = "requires a reachable PostgreSQL test database"]
async fn snapshot_items_cover_members_roles_service_accounts_and_oauth_clients() {
    let _guard = crate::test_support::test_database_lock().lock().await;
    let pool = crate::test_support::isolated_test_pool(5);
    crate::test_support::ensure_test_database(&pool).await;
    let fixture = seed_access_review_fixture(&pool).await;

    let mut tx = pool.begin().await.expect("transaction should start");
    let campaign_id = insert_campaign(
        &mut tx,
        fixture.tenant_id,
        fixture.admin_principal_id,
        "Quarterly access review",
        Some("Q2 controls"),
        Utc::now() + Duration::days(7),
    )
    .await
    .expect("campaign should be inserted");

    let item_count = insert_snapshot_items(
        &mut tx,
        campaign_id,
        fixture.tenant_id,
        &AccessReviewCampaignScopeInput {
            include_members: true,
            include_roles: true,
            include_service_accounts: true,
            include_oauth_clients: true,
        },
    )
    .await
    .expect("snapshot should be inserted");
    tx.commit()
        .await
        .expect("snapshot transaction should commit");

    assert_eq!(item_count, 7);

    let item_counts = sqlx::query(
        r#"
        SELECT item_type::text AS item_type, COUNT(*)::bigint AS count
        FROM access_review_items
        WHERE campaign_id = $1
        GROUP BY item_type
        ORDER BY item_type::text
        "#,
    )
    .bind(campaign_id)
    .fetch_all(&pool)
    .await
    .expect("snapshot counts should be readable");

    let counts = item_counts
        .iter()
        .map(|row| {
            (
                row.get::<String, _>("item_type"),
                row.get::<i64, _>("count"),
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();

    assert_eq!(counts.get("member"), Some(&2));
    assert_eq!(counts.get("oauth_client"), Some(&1));
    assert_eq!(counts.get("role"), Some(&3));
    assert_eq!(counts.get("service_account"), Some(&1));

    cleanup_fixture(&pool, fixture.tenant_id).await;
}

struct AccessReviewDbFixture {
    tenant_id: Uuid,
    admin_principal_id: Uuid,
}

async fn seed_access_review_fixture(pool: &PgPool) -> AccessReviewDbFixture {
    let tenant_id = Uuid::new_v4();
    let workspace_id = Uuid::new_v4();
    let admin_principal_id = Uuid::new_v4();
    let member_principal_id = Uuid::new_v4();
    let service_principal_id = Uuid::new_v4();
    let client_uuid = Uuid::new_v4();
    let now = Utc::now();
    let tenant_slug = format!("access-review-test-{tenant_id}");
    let admin_email = format!("access-review-admin-{admin_principal_id}@example.com");
    let member_email = format!("access-review-member-{member_principal_id}@example.com");
    let service_name = format!("Access Review Service {}", service_principal_id.simple());
    let client_id = format!("ar_{}", client_uuid.simple());

    sqlx::query(
        r#"
        INSERT INTO tenants (id, kind, name, slug, status, security_tier, created_at, updated_at)
        VALUES ($1, 'team', 'Access Review Test Tenant', $2, 'active', 'standard', $3, $3)
        "#,
    )
    .bind(tenant_id)
    .bind(&tenant_slug)
    .bind(now)
    .execute(pool)
    .await
    .expect("tenant insert should succeed");

    insert_human_principal(pool, tenant_id, admin_principal_id, &admin_email, "Admin").await;
    insert_human_principal(
        pool,
        tenant_id,
        member_principal_id,
        &member_email,
        "Member",
    )
    .await;

    sqlx::query(
        r#"
        INSERT INTO workspaces (
          id, tenant_id, name, workspace_type, owner_user_id, plan_code, created_at, updated_at
        )
        VALUES ($1, $2, 'Access Review Workspace', 'team', $3, 'team', $4, $4)
        "#,
    )
    .bind(workspace_id)
    .bind(tenant_id)
    .bind(admin_principal_id)
    .bind(now)
    .execute(pool)
    .await
    .expect("workspace insert should succeed");

    sqlx::query(
        r#"
        INSERT INTO tenant_memberships (tenant_id, principal_id, principal_kind, role, status, source, created_at, updated_at)
        VALUES
          ($1, $2, 'human', 'owner', 'active', 'manual', $4, $4),
          ($1, $3, 'human', 'member', 'active', 'manual', $4, $4)
        "#,
    )
    .bind(tenant_id)
    .bind(admin_principal_id)
    .bind(member_principal_id)
    .bind(now)
    .execute(pool)
    .await
    .expect("tenant memberships insert should succeed");

    sqlx::query(
        r#"
        INSERT INTO principals (id, tenant_id, principal_kind, status, display_name, created_at, updated_at)
        VALUES ($1, $2, 'service_account', 'active', $3, $4, $4)
        "#,
    )
    .bind(service_principal_id)
    .bind(tenant_id)
    .bind(&service_name)
    .bind(now)
    .execute(pool)
    .await
    .expect("service principal insert should succeed");

    sqlx::query(
        r#"
        INSERT INTO workspace_memberships (workspace_id, principal_id, role, status, source, created_at, updated_at)
        VALUES
          ($1, $2, 'owner', 'active', 'manual', $5, $5),
          ($1, $3, 'member', 'active', 'manual', $5, $5),
          ($1, $4, 'viewer', 'active', 'system', $5, $5)
        "#,
    )
    .bind(workspace_id)
    .bind(admin_principal_id)
    .bind(member_principal_id)
    .bind(service_principal_id)
    .bind(now)
    .execute(pool)
    .await
    .expect("workspace memberships insert should succeed");

    sqlx::query(
        r#"
        INSERT INTO service_accounts (
          principal_id, tenant_id, workspace_id, created_by_principal_id, name,
          description, auth_method, client_id, last_rotated_at, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, 'snapshot test', 'oauth_client_credentials', $6, $7, $7, $7)
        "#,
    )
    .bind(service_principal_id)
    .bind(tenant_id)
    .bind(workspace_id)
    .bind(admin_principal_id)
    .bind(&service_name)
    .bind(&client_id)
    .bind(now)
    .execute(pool)
    .await
    .expect("service account insert should succeed");

    sqlx::query(
        r#"
        INSERT INTO oauth_clients (
          id, client_id, client_secret_hash, name, redirect_uris, tenant_id,
          owner_scope_type, owner_scope_id, client_type, revoked_at, created_at, updated_at
        )
        VALUES ($1, $2, 'test-secret-hash', 'Access Review OAuth Client',
          ARRAY['https://example.com/callback'], $3, 'workspace', $4, 'service', NULL, $5, $5)
        "#,
    )
    .bind(client_uuid)
    .bind(&client_id)
    .bind(tenant_id)
    .bind(workspace_id)
    .bind(now)
    .execute(pool)
    .await
    .expect("oauth client insert should succeed");

    AccessReviewDbFixture {
        tenant_id,
        admin_principal_id,
    }
}

async fn insert_human_principal(
    pool: &PgPool,
    tenant_id: Uuid,
    principal_id: Uuid,
    email: &str,
    display_name: &str,
) {
    let now = Utc::now();
    sqlx::query(
        r#"
        INSERT INTO principals (id, tenant_id, principal_kind, status, display_name, created_at, updated_at)
        VALUES ($1, $2, 'human', 'active', $3, $4, $4)
        "#,
    )
    .bind(principal_id)
    .bind(tenant_id)
    .bind(display_name)
    .bind(now)
    .execute(pool)
    .await
    .expect("principal insert should succeed");

    sqlx::query(
        r#"
        INSERT INTO users (
          principal_id, email, name, firstname, lastname, username, password_hash,
          email_verified_at, status, created_at, updated_at
        )
        VALUES ($1, $2, $3, $3, 'Reviewer', $4, 'test-password-hash',
          $5, 'active', $5, $5)
        "#,
    )
    .bind(principal_id)
    .bind(email)
    .bind(display_name)
    .bind(format!("user-{principal_id}"))
    .bind(now)
    .execute(pool)
    .await
    .expect("user insert should succeed");
}

async fn cleanup_fixture(pool: &PgPool, tenant_id: Uuid) {
    sqlx::query("DELETE FROM tenants WHERE id = $1")
        .bind(tenant_id)
        .execute(pool)
        .await
        .expect("tenant cleanup should succeed");
}
