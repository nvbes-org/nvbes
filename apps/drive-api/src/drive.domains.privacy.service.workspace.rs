use sqlx::PgPool;

use crate::{domains::authz::WorkspaceAccess, http::error::AppError};
use nvbes_tenancy::RlsContext;

use super::payload::privacy_request_payload;
use super::policy::ensure_workspace_delete_allowed;
use super::requests::create_privacy_request;
use super::*;

pub async fn request_workspace_export(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    access: &WorkspaceAccess,
) -> Result<PrivacyRequestResponse, AppError> {
    create_privacy_request(
        db,
        redis,
        PrivacyRequestDraft {
            request_type: "workspace_export",
            job_type: JOB_PRIVACY_WORKSPACE_EXPORT,
            subject_user_id: None,
            workspace_id: Some(access.workspace_id),
            requested_by: access.auth.user_id,
            requested_by_principal_id: access.auth.principal_id,
            rls_context: RlsContext {
                principal_id: Some(access.auth.principal_id),
                user_id: Some(access.auth.user_id),
                tenant_id: access.tenant_id.or(access.auth.tenant_id),
                workspace_id: Some(access.workspace_id),
            },
            payload: privacy_request_payload(
                None,
                Some(access.workspace_id),
                access.auth.user_id,
                access.auth.principal_id,
            ),
            message: "Workspace data export has been queued.",
        },
    )
    .await
}

pub async fn request_workspace_delete(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    access: &WorkspaceAccess,
) -> Result<PrivacyRequestResponse, AppError> {
    ensure_workspace_delete_allowed(db, access.workspace_id).await?;

    create_privacy_request(
        db,
        redis,
        PrivacyRequestDraft {
            request_type: "workspace_delete",
            job_type: JOB_PRIVACY_WORKSPACE_DELETE,
            subject_user_id: None,
            workspace_id: Some(access.workspace_id),
            requested_by: access.auth.user_id,
            requested_by_principal_id: access.auth.principal_id,
            rls_context: RlsContext {
                principal_id: Some(access.auth.principal_id),
                user_id: Some(access.auth.user_id),
                tenant_id: access.tenant_id.or(access.auth.tenant_id),
                workspace_id: Some(access.workspace_id),
            },
            payload: privacy_request_payload(
                None,
                Some(access.workspace_id),
                access.auth.user_id,
                access.auth.principal_id,
            ),
            message: "Workspace deletion has been queued.",
        },
    )
    .await
}
