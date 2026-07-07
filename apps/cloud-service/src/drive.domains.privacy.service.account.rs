use sqlx::PgPool;
use uuid::Uuid;

use crate::{domains::auth::types::AuthContext, http::error::AppError};
use nvbes_tenancy::RlsContext;

use super::super::db;
use super::download::attach_export_download;
use super::payload::privacy_request_payload;
use super::policy::ensure_account_delete_allowed;
use super::requests::create_privacy_request;
use super::*;

pub async fn get_request(
    db: &PgPool,
    storage: &dyn nvbes_storage::ObjectStore,
    auth: &AuthContext,
    request_id: Uuid,
) -> Result<PrivacyRequestStatusResponse, AppError> {
    let request =
        db::fetch_privacy_request(db, auth.user_id, auth.principal_id, request_id).await?;

    let mut request = request.ok_or_else(|| {
        AppError::not_found("privacy_request_not_found", "Privacy request not found.")
    })?;

    attach_export_download(storage, &mut request).await?;

    Ok(request)
}

pub async fn request_account_export(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    auth: &AuthContext,
) -> Result<PrivacyRequestResponse, AppError> {
    create_privacy_request(
        db,
        redis,
        PrivacyRequestDraft {
            request_type: "account_export",
            job_type: JOB_PRIVACY_ACCOUNT_EXPORT,
            subject_user_id: Some(auth.user_id),
            workspace_id: None,
            requested_by: auth.user_id,
            requested_by_principal_id: auth.principal_id,
            rls_context: RlsContext {
                principal_id: Some(auth.principal_id),
                user_id: Some(auth.user_id),
                tenant_id: auth.tenant_id,
                workspace_id: auth.workspace_id,
            },
            payload: privacy_request_payload(
                Some(auth.user_id),
                None,
                auth.user_id,
                auth.principal_id,
            ),
            message: "Account data export has been queued.",
        },
    )
    .await
}

pub async fn request_account_delete(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    auth: &AuthContext,
) -> Result<PrivacyRequestResponse, AppError> {
    ensure_account_delete_allowed(db, auth.user_id).await?;

    create_privacy_request(
        db,
        redis,
        PrivacyRequestDraft {
            request_type: "account_delete",
            job_type: JOB_PRIVACY_ACCOUNT_DELETE,
            subject_user_id: Some(auth.user_id),
            workspace_id: None,
            requested_by: auth.user_id,
            requested_by_principal_id: auth.principal_id,
            rls_context: RlsContext {
                principal_id: Some(auth.principal_id),
                user_id: Some(auth.user_id),
                tenant_id: auth.tenant_id,
                workspace_id: auth.workspace_id,
            },
            payload: privacy_request_payload(
                Some(auth.user_id),
                None,
                auth.user_id,
                auth.principal_id,
            ),
            message: "Account deletion has been queued.",
        },
    )
    .await
}
