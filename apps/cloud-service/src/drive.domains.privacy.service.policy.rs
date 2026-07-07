use sqlx::PgPool;
use uuid::Uuid;

use crate::http::error::AppError;

use super::super::db;

pub(super) async fn ensure_account_delete_allowed(
    db: &PgPool,
    user_id: Uuid,
) -> Result<(), AppError> {
    let owned_workspace_count = db::count_owned_workspaces(db, user_id).await?;

    if owned_workspace_count > 0 {
        return Err(AppError::conflict(
            "account_owns_workspaces",
            "Delete or transfer owned workspaces before deleting this account.",
        ));
    }

    Ok(())
}

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
