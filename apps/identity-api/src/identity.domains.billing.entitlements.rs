use crate::http::error::AppError;
use sqlx::PgPool;
use uuid::Uuid;

use super::service::get_billing;

pub async fn get_entitlements_for_workspace(
    db: &PgPool,
    workspace_id: Uuid,
) -> Result<nvbes_billing::types::ProductEntitlementsView, AppError> {
    let billing_response = get_billing(db, workspace_id).await?;
    Ok(billing_response.entitlements)
}
