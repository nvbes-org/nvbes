use sqlx::PgPool;
use uuid::Uuid;

use crate::{domains::auth::types::AuthContext, http::error::AppError};

use super::super::db;
use super::download::attach_export_download;
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
