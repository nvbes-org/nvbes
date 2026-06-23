use nvbes_redis::worker_queue::QueuedJob;

use crate::db::Database;

pub(super) async fn execute_job(
    job: &QueuedJob,
    database: &Database,
    storage: &dyn nvbes_storage::ObjectStore,
) -> anyhow::Result<serde_json::Value> {
    if !is_known_job_type(job.job_type.as_str()) {
        anyhow::bail!("unknown job type: {}", job.job_type);
    }

    match job.job_type.as_str() {
        super::super::maintenance::JOB_UPLOADS_PURGE_EXPIRED => {
            super::super::maintenance::purge_expired_uploads(database, storage).await
        }
        super::super::maintenance::JOB_QUOTAS_RECALCULATE => {
            super::super::maintenance::recalculate_quotas(database).await
        }
        super::super::maintenance::JOB_TRASH_PURGE => {
            super::super::maintenance::purge_trash(database).await
        }
        super::super::maintenance::JOB_STORAGE_PURGE_DELETED => {
            super::super::maintenance::storage::purge_deleted_storage(database, storage).await
        }
        super::super::maintenance::JOB_STORAGE_PURGE_QUARANTINED => {
            super::super::maintenance::storage::purge_quarantined(database, storage, 30).await
        }
        super::super::maintenance::JOB_GEO_LOOKUP_MAINTENANCE => {
            super::super::maintenance::run_geo_lookup_maintenance(database).await
        }
        super::super::privacy::delete::JOB_PRIVACY_ACCOUNT_DELETE => {
            super::super::privacy::delete::delete_account_data(job, database).await
        }
        super::super::privacy::delete::JOB_PRIVACY_WORKSPACE_DELETE => {
            super::super::privacy::delete::delete_workspace_data(job, database, storage).await
        }
        super::super::privacy::export::JOB_PRIVACY_ACCOUNT_EXPORT => {
            super::super::privacy::export::export_account_data(job, database, storage).await
        }
        super::super::privacy::export::JOB_PRIVACY_WORKSPACE_EXPORT => {
            super::super::privacy::export::export_workspace_data(job, database, storage).await
        }
        _ => unreachable!("job type was already validated"),
    }
}

pub(super) fn is_known_job_type(job_type: &str) -> bool {
    matches!(
        job_type,
        super::super::maintenance::JOB_UPLOADS_PURGE_EXPIRED
            | super::super::maintenance::JOB_QUOTAS_RECALCULATE
            | super::super::maintenance::JOB_TRASH_PURGE
            | super::super::maintenance::JOB_STORAGE_PURGE_DELETED
            | super::super::maintenance::JOB_STORAGE_PURGE_QUARANTINED
            | super::super::maintenance::JOB_GEO_LOOKUP_MAINTENANCE
            | super::super::privacy::delete::JOB_PRIVACY_ACCOUNT_DELETE
            | super::super::privacy::delete::JOB_PRIVACY_WORKSPACE_DELETE
            | super::super::privacy::export::JOB_PRIVACY_ACCOUNT_EXPORT
            | super::super::privacy::export::JOB_PRIVACY_WORKSPACE_EXPORT
    )
}
