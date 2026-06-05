use serde_json::Value as JsonValue;
use uuid::Uuid;

use crate::db::Database;
use crate::workers::logic;
use nvbes_redis::worker_queue::QueuedJob;

pub const JOB_PRIVACY_ACCOUNT_DELETE: &str = "privacy.account_delete";
pub const JOB_PRIVACY_WORKSPACE_DELETE: &str = "privacy.workspace_delete";

pub async fn delete_account_data(
    job: &QueuedJob,
    database: &Database,
) -> anyhow::Result<JsonValue> {
    let user_id: Uuid = logic::payload_uuid(&job.payload, "user_id")?;

    let mut tx = database.begin().await?;

    sqlx::query("DELETE FROM workspace_memberships WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query("UPDATE users SET status = 'deleted', updated_at = NOW() WHERE id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(serde_json::json!({ "user_id": user_id }))
}

pub async fn delete_workspace_data(
    job: &QueuedJob,
    database: &Database,
    storage: &dyn nvbes_storage::ObjectStore,
) -> anyhow::Result<JsonValue> {
    let workspace_id: Uuid = logic::payload_uuid(&job.payload, "workspace_id")?;

    let mut tx = database.begin().await?;

    sqlx::query("DELETE FROM workspace_memberships WHERE workspace_id = $1")
        .bind(workspace_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query(
        r#"
        UPDATE storage_objects
        SET status = 'deleted',
            updated_at = NOW()
        WHERE workspace_id = $1
        "#,
    )
    .bind(workspace_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    let mut deleted_metadata = 0_u64;
    let mut deleted_storage_objects = 0_usize;
    loop {
        let result = crate::workers::maintenance::storage::purge_workspace_deleted_storage(
            database,
            storage,
            workspace_id,
        )
        .await?;

        if result.deleted_metadata == 0 {
            break;
        }

        deleted_metadata += result.deleted_metadata;
        deleted_storage_objects += result.deleted_storage_objects;
    }

    let mut tx = database.begin().await?;

    sqlx::query("UPDATE workspaces SET deleted_at = NOW(), updated_at = NOW() WHERE id = $1")
        .bind(workspace_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(serde_json::json!({
        "workspace_id": workspace_id,
        "deleted_objects": deleted_metadata,
        "storage_objects": deleted_storage_objects
    }))
}
