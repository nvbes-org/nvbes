use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::list_trash;
use crate::{
    domains::files::types::{ObjectTypeFilter, TrashListInput},
    test_support::{seed_workspace, test_pool, workspace_access},
};

async fn supports_current_storage_schema(pool: &PgPool) -> bool {
    sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
          SELECT 1 FROM information_schema.columns
          WHERE table_name = 'users' AND column_name = 'display_name'
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

async fn seed_trashed_folder(
    pool: &PgPool,
    workspace_id: Uuid,
    principal_id: Uuid,
    name: &str,
    minutes_ago: i64,
) {
    sqlx::query(
        r#"
        INSERT INTO storage_objects (
          workspace_id, object_type, name, size_bytes, status,
          created_by, created_by_principal_id, trashed_at
        )
        VALUES ($1, 'folder', $2, 0, 'trashed', $3, $3, $4)
        "#,
    )
    .bind(workspace_id)
    .bind(name)
    .bind(principal_id)
    .bind(Utc::now() - Duration::minutes(minutes_ago))
    .execute(pool)
    .await
    .expect("trashed folder insert should succeed");
}

#[tokio::test]
async fn trash_pages_are_stable_and_filtered() {
    let pool = test_pool();
    if !supports_current_storage_schema(&pool).await {
        eprintln!("skipping test: local database is missing the current Cloud storage schema");
        return;
    }
    let key = format!("trash-pagination-{}", Uuid::new_v4());
    let (principal_id, workspace_id, tenant_id) = seed_workspace(&pool, &key).await;
    seed_trashed_folder(&pool, workspace_id, principal_id, "Report_2026", 1).await;
    seed_trashed_folder(&pool, workspace_id, principal_id, "ReportX2026", 2).await;
    seed_trashed_folder(&pool, workspace_id, principal_id, "Report_2025", 3).await;
    let access = workspace_access(principal_id, workspace_id, tenant_id);

    let first = list_trash(
        &pool,
        &access,
        TrashListInput {
            limit: Some(1),
            cursor: None,
            object_type: Some(ObjectTypeFilter::Folder),
            name_prefix: Some("report_".to_string()),
        },
    )
    .await
    .expect("first trash page should load");
    assert_eq!(first.objects.len(), 1);
    assert!(first.has_more);

    let second = list_trash(
        &pool,
        &access,
        TrashListInput {
            limit: Some(1),
            cursor: first.next_cursor,
            object_type: Some(ObjectTypeFilter::Folder),
            name_prefix: Some("report_".to_string()),
        },
    )
    .await
    .expect("second trash page should load");
    assert_eq!(second.objects.len(), 1);
    assert!(!second.has_more);
    assert!(second.next_cursor.is_none());
}
