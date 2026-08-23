use sqlx::PgPool;
use uuid::Uuid;

use crate::http::error::AppError;

use super::super::db;

pub(super) async fn ensure_workspace_delete_allowed(
    db: &PgPool,
    workspace_id: Uuid,
) -> Result<(), AppError> {
    let legal_hold = db::check_legal_hold(db, workspace_id).await?;

    if legal_hold {
        return Err(AppError::conflict(
            "workspace_legal_hold",
            "Workspace deletion is blocked by an active legal hold.",
        ));
    }

    Ok(())
}
