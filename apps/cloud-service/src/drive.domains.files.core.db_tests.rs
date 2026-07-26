use sqlx::PgPool;
use uuid::Uuid;

use super::list_objects;
use crate::{
    domains::files::types::{ListObjectsInput, ObjectTypeFilter},
    test_support::{seed_workspace, test_pool, workspace_access},
};

async fn seed_folder(pool: &PgPool, workspace_id: Uuid, principal_id: Uuid, name: &str) {
    sqlx::query(
        r#"
        INSERT INTO storage_objects (
          workspace_id, object_type, name, size_bytes, status,
          created_by, created_by_principal_id
        )
        VALUES ($1, 'folder', $2, 0, 'active', $3, $3)
        "#,
    )
    .bind(workspace_id)
    .bind(name)
    .bind(principal_id)
    .execute(pool)
    .await
    .expect("folder insert should succeed");
}

async fn supports_current_storage_schema(pool: &PgPool) -> bool {
    sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
          SELECT 1 FROM information_schema.columns
          WHERE table_name = 'users' AND column_name = 'id'
        ) AND EXISTS (
          SELECT 1 FROM information_schema.columns
          WHERE table_name = 'storage_objects' AND column_name = 'created_by_principal_id'
        )
        "#,
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false)
}

#[tokio::test]
async fn object_pages_are_stable_and_report_the_final_page() {
    let pool = test_pool();
    if !supports_current_storage_schema(&pool).await {
        eprintln!("skipping test: local database is missing the current Cloud storage schema");
        return;
    }
    let key = format!("object-pagination-{}", Uuid::new_v4());
    let (principal_id, workspace_id, tenant_id) = seed_workspace(&pool, &key).await;
    for name in ["Alpha", "beta", "Gamma"] {
        seed_folder(&pool, workspace_id, principal_id, name).await;
    }
    let access = workspace_access(principal_id, workspace_id, tenant_id);

    let first = list_objects(
        &pool,
        &access,
        ListObjectsInput {
            parent_id: None,
            limit: Some(2),
            cursor: None,
            object_type: Some(ObjectTypeFilter::Folder),
            name_prefix: None,
        },
    )
    .await
    .expect("first page should load");
    assert_eq!(first.objects.len(), 2);
    assert!(first.has_more);

    let second = list_objects(
        &pool,
        &access,
        ListObjectsInput {
            parent_id: None,
            limit: Some(2),
            cursor: first.next_cursor,
            object_type: Some(ObjectTypeFilter::Folder),
            name_prefix: None,
        },
    )
    .await
    .expect("second page should load");
    assert_eq!(second.objects.len(), 1);
    assert!(!second.has_more);
    assert!(second.next_cursor.is_none());
}

#[tokio::test]
async fn object_name_prefix_filter_is_literal_and_case_insensitive() {
    let pool = test_pool();
    if !supports_current_storage_schema(&pool).await {
        eprintln!("skipping test: local database is missing the current Cloud storage schema");
        return;
    }
    let key = format!("object-filter-{}", Uuid::new_v4());
    let (principal_id, workspace_id, tenant_id) = seed_workspace(&pool, &key).await;
    seed_folder(&pool, workspace_id, principal_id, "Budget_2026").await;
    seed_folder(&pool, workspace_id, principal_id, "BudgetX2026").await;
    let access = workspace_access(principal_id, workspace_id, tenant_id);

    let page = list_objects(
        &pool,
        &access,
        ListObjectsInput {
            parent_id: None,
            limit: Some(10),
            cursor: None,
            object_type: Some(ObjectTypeFilter::Folder),
            name_prefix: Some("budget_".to_string()),
        },
    )
    .await
    .expect("filtered page should load");

    assert_eq!(page.objects.len(), 1);
    assert_eq!(page.objects[0].name, "Budget_2026");
}
