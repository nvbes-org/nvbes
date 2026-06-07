use sqlx::PgPool;

use crate::http::error::AppError;

use super::super::db;
use super::payload::{merge_request_payload, privacy_idempotency_key};
use super::{PrivacyRequestDraft, PrivacyRequestResponse};

pub(super) async fn create_privacy_request(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    draft: PrivacyRequestDraft,
) -> Result<PrivacyRequestResponse, AppError> {
    let mut tx = db.begin().await?;
    nvbes_tenancy::set_transaction_rls_context(&mut tx, draft.rls_context).await?;
    let idempotency_key = privacy_idempotency_key(&draft);

    let response = db::insert_privacy_request_tx(&mut tx, &draft).await?;
    let worker_job_id = response.worker_job_id.unwrap_or_else(uuid::Uuid::new_v4);
    db::update_privacy_request_job_tx(&mut tx, response.id, worker_job_id).await?;
    tx.commit().await?;

    let payload = merge_request_payload(draft.payload, response.id);
    db::insert_worker_job_tx(
        redis,
        draft.job_type,
        draft.job_type,
        &idempotency_key,
        payload,
        worker_job_id,
    )
    .await?;

    Ok(PrivacyRequestResponse {
        request_id: response.id,
        request_type: draft.request_type.to_string(),
        status: response.status,
        worker_job_id,
        requested_by_principal_id: response.requested_by_principal_id,
        message: draft.message.to_string(),
        requested_at: response.requested_at,
        guardrails: vec![
            "Identity is verified through the active authenticated session.".to_string(),
            "Audit logs, billing records and security logs are retained when legally required."
                .to_string(),
            "Deletion jobs anonymize retained references instead of breaking audit integrity."
                .to_string(),
            "Backup purges must respect documented retention and replay completed deletion requests."
                .to_string(),
        ],
    })
}
