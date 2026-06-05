use serde_json::Value as JsonValue;
use std::time::Duration;
use uuid::Uuid;

#[path = "drive.workers.privacy.export.account.rs"]
mod account;
#[path = "drive.workers.privacy.export.workspace.rs"]
mod workspace;

use crate::db::Database;
use crate::workers::logic;
use nvbes_redis::worker_queue::QueuedJob;

pub const JOB_PRIVACY_ACCOUNT_EXPORT: &str = "privacy.account_export";
pub const JOB_PRIVACY_WORKSPACE_EXPORT: &str = "privacy.workspace_export";

pub async fn export_account_data(
    job: &QueuedJob,
    database: &Database,
    storage: &dyn nvbes_storage::ObjectStore,
) -> anyhow::Result<JsonValue> {
    let subject_user_id: Uuid = logic::payload_uuid(&job.payload, "subject_user_id")?;
    let request_id: Uuid = logic::payload_uuid(&job.payload, "privacy_request_id")?;

    mark_processing(database, request_id).await?;
    let export = account::build(database, subject_user_id).await?;
    store_export_result(database, storage, request_id, "drive-account", export).await
}

pub async fn export_workspace_data(
    job: &QueuedJob,
    database: &Database,
    storage: &dyn nvbes_storage::ObjectStore,
) -> anyhow::Result<JsonValue> {
    let workspace_id: Uuid = logic::payload_uuid(&job.payload, "workspace_id")?;
    let request_id: Uuid = logic::payload_uuid(&job.payload, "privacy_request_id")?;

    mark_processing(database, request_id).await?;
    let export = workspace::build(database, workspace_id).await?;
    store_export_result(database, storage, request_id, "drive-workspace", export).await
}

async fn mark_processing(database: &Database, request_id: Uuid) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        UPDATE privacy_requests
        SET status = 'processing', updated_at = NOW()
        WHERE id = $1 AND status = 'queued'
        "#,
    )
    .bind(request_id)
    .execute(&**database)
    .await?;

    Ok(())
}

async fn store_export_result(
    database: &Database,
    storage: &dyn nvbes_storage::ObjectStore,
    request_id: Uuid,
    prefix: &str,
    export: JsonValue,
) -> anyhow::Result<JsonValue> {
    let object_key = format!("privacy-exports/{prefix}/{request_id}.json");
    let body = serde_json::to_vec_pretty(&export)?;
    let size_bytes = body.len();

    storage
        .put_object(&object_key, Some("application/json"), body)
        .await?;

    let result = serde_json::json!({
        "delivery": {
            "type": "authenticated_presigned_download",
            "object_key": object_key,
            "content_type": "application/json",
            "size_bytes": size_bytes,
            "download_expires_in_seconds": 900
        },
        "coverage": {
            "file_metadata_included": true,
            "file_content_included": false,
            "secrets_excluded": true
        }
    });

    sqlx::query(
        r#"
        UPDATE privacy_requests
        SET status = 'completed',
            result = $2,
            completed_at = NOW(),
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(request_id)
    .bind(sqlx::types::Json(result.clone()))
    .execute(&**database)
    .await?;

    let delivery = result
        .get("delivery")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let mut public_result = serde_json::json!({
        "status": "completed",
        "privacy_request_id": request_id,
        "delivery": delivery
    });

    attach_download(storage, &mut public_result).await?;
    Ok(public_result)
}

async fn attach_download(
    storage: &dyn nvbes_storage::ObjectStore,
    result: &mut JsonValue,
) -> anyhow::Result<()> {
    let Some(delivery) = result
        .get_mut("delivery")
        .and_then(JsonValue::as_object_mut)
    else {
        return Ok(());
    };
    let Some(object_key) = delivery
        .get("object_key")
        .and_then(JsonValue::as_str)
        .map(str::to_owned)
    else {
        return Ok(());
    };

    let expires_in = delivery
        .get("download_expires_in_seconds")
        .and_then(JsonValue::as_u64)
        .unwrap_or(900)
        .clamp(60, 3600);
    let download = storage
        .presign_download(&object_key, Duration::from_secs(expires_in))
        .await?;

    delivery.remove("object_key");
    delivery.insert(
        "download".to_owned(),
        serde_json::json!({
            "url": download.url,
            "method": download.method,
            "expires_in_seconds": download.expires_in.as_secs()
        }),
    );

    Ok(())
}
