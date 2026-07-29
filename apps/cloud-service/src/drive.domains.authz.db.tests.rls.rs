use uuid::Uuid;

use super::support::*;

const SECURITY_MIGRATION: &str = include_str!("../migrations/0019_storage_security_hardening.sql");
const CLOUD_FORCED_RLS_TABLES: [&str; 23] = [
    "api_key_nonces",
    "api_keys",
    "api_request_logs",
    "audit_events",
    "billing_entitlement_snapshots",
    "organization_memberships",
    "organizations",
    "privacy_requests",
    "quota_usage",
    "sessions",
    "share_links",
    "storage_object_key_envelopes",
    "storage_objects",
    "tenant_memberships",
    "tenants",
    "upload_parts",
    "upload_sessions",
    "usage_events",
    "workspace_invitations",
    "workspace_members",
    "workspace_memberships",
    "workspace_policies",
    "workspaces",
];

#[test]
fn migration_defines_non_owner_roles_and_forced_rls() {
    assert!(SECURITY_MIGRATION.contains("CREATE ROLE nvbes_app"));
    assert!(SECURITY_MIGRATION.contains("NOBYPASSRLS"));
    assert!(SECURITY_MIGRATION.contains("ALTER TABLE storage_objects FORCE ROW LEVEL SECURITY"));
    assert!(
        SECURITY_MIGRATION.contains("ALTER TABLE workspace_memberships FORCE ROW LEVEL SECURITY")
    );
}

#[test]
fn migration_defines_retention_and_cryptographic_erasure() {
    assert!(SECURITY_MIGRATION.contains("CREATE TABLE data_retention_policies"));
    assert!(SECURITY_MIGRATION.contains("CREATE TABLE storage_object_key_envelopes"));
    assert!(SECURITY_MIGRATION.contains("cryptographically_erase_storage_object"));
}

#[tokio::test]
async fn application_role_cannot_read_or_write_another_workspace() {
    let pool = test_pool();
    if !db_supports_current_drive_schema(&pool).await || !application_role_exists(&pool).await {
        eprintln!("skipping test: current RLS migration is not installed");
        return;
    }

    let first_key = format!("drive-rls-a-{}", Uuid::new_v4());
    let second_key = format!("drive-rls-b-{}", Uuid::new_v4());
    let (first_user, first_workspace, first_tenant) = seed_workspace(&pool, &first_key).await;
    let (second_user, second_workspace, _) = seed_workspace(&pool, &second_key).await;
    let first_object = seed_object(&pool, first_workspace, first_user, "first.txt").await;
    let second_object = seed_object(&pool, second_workspace, second_user, "second.txt").await;

    let mut tx = pool.begin().await.expect("transaction should begin");
    sqlx::query("SET LOCAL ROLE nvbes_app")
        .execute(&mut *tx)
        .await
        .expect("migration owner should be allowed to assume nvbes_app");
    nvbes_tenancy::set_transaction_rls_context(
        &mut tx,
        nvbes_tenancy::RlsContext {
            principal_id: Some(first_user),
            user_id: Some(first_user),
            tenant_id: Some(first_tenant),
            workspace_id: Some(first_workspace),
        },
    )
    .await
    .expect("RLS context should be set");

    let visible_ids = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM storage_objects WHERE id = ANY($1) ORDER BY id",
    )
    .bind(vec![first_object, second_object])
    .fetch_all(&mut *tx)
    .await
    .expect("RLS filtered read should succeed");
    assert_eq!(visible_ids, vec![first_object]);

    let cross_tenant_insert = sqlx::query(
        r#"
        INSERT INTO storage_objects (
          id, workspace_id, object_type, name, size_bytes, mime_type, object_key,
          status, created_by, created_by_principal_id
        )
        VALUES ($1, $2, 'file', 'forbidden.txt', 1, 'text/plain', $3, 'active', $4, $4)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(second_workspace)
    .bind(format!("tests/{}", Uuid::new_v4()))
    .bind(first_user)
    .execute(&mut *tx)
    .await;
    assert!(cross_tenant_insert.is_err());
    tx.rollback().await.ok();

    cleanup(&pool, &first_key).await;
    cleanup(&pool, &second_key).await;
}

#[tokio::test]
async fn cloud_tenant_tables_force_rls_even_in_a_shared_schema() {
    let pool = test_pool();
    if !application_role_exists(&pool).await {
        eprintln!("skipping test: current RLS migration is not installed");
        return;
    }

    let missing_or_unforced_tables = sqlx::query_scalar::<_, String>(
        r#"
        SELECT expected.table_name
        FROM unnest($1::text[]) AS expected(table_name)
        LEFT JOIN pg_class AS class
          ON class.relname = expected.table_name
         AND class.relnamespace = 'public'::regnamespace
         AND class.relkind IN ('r', 'p')
        WHERE class.oid IS NULL
           OR NOT class.relrowsecurity
           OR NOT class.relforcerowsecurity
        ORDER BY expected.table_name
        "#,
    )
    .bind(CLOUD_FORCED_RLS_TABLES.as_slice())
    .fetch_all(&pool)
    .await
    .expect("RLS metadata query should succeed");

    assert!(
        missing_or_unforced_tables.is_empty(),
        "Cloud contract tables must exist and use FORCE ROW LEVEL SECURITY: \
         {missing_or_unforced_tables:?}"
    );
}

async fn application_role_exists(pool: &sqlx::PgPool) -> bool {
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'nvbes_app')",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false)
}

async fn seed_object(pool: &sqlx::PgPool, workspace_id: Uuid, user_id: Uuid, name: &str) -> Uuid {
    let object_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO storage_objects (
          id, workspace_id, object_type, name, size_bytes, mime_type, object_key,
          status, created_by, created_by_principal_id
        )
        VALUES ($1, $2, 'file', $3, 1, 'text/plain', $4, 'active', $5, $5)
        "#,
    )
    .bind(object_id)
    .bind(workspace_id)
    .bind(name)
    .bind(format!("tests/{workspace_id}/{object_id}"))
    .bind(user_id)
    .execute(pool)
    .await
    .expect("storage object insert should succeed");
    object_id
}
