use super::routes_access::PublicApiRequestContext;
use super::types::PublicApiAuditEventInput;
use crate::http::error::AppError;
use sqlx::PgPool;
use uuid::Uuid;

pub async fn record_api_event(
    db: &PgPool,
    request: &PublicApiRequestContext,
    action: &str,
    target_type: &str,
    target_id: Option<Uuid>,
    metadata: serde_json::Value,
) -> Result<(), AppError> {
    crate::domains::public_api::record_api_audit_event(
        db,
        &request.ctx,
        PublicApiAuditEventInput {
            action,
            target_type,
            target_id,
            ip: request.meta.ip(),
            user_agent: request.meta.user_agent(),
            metadata,
        },
    )
    .await
}
