use axum::http::HeaderMap;
use sqlx::PgPool;
use uuid::Uuid;

use crate::billing_admin_types::BackofficeAccess;
use crate::error::AppError;

const ACTOR_HEADER: &str = "x-nvbes-actor-principal-id";

pub(crate) async fn authorize_backoffice(
    db: &PgPool,
    headers: &HeaderMap,
    workspace_id: Uuid,
) -> Result<BackofficeAccess, AppError> {
    let actor_principal_id = actor_principal_id(headers)?;
    let tenant_id = sqlx::query_scalar::<_, Uuid>("SELECT tenant_id FROM workspaces WHERE id = $1")
        .bind(workspace_id)
        .fetch_one(db)
        .await?;
    Ok(BackofficeAccess {
        tenant_id,
        actor_principal_id,
    })
}

pub(crate) fn actor_principal_id(headers: &HeaderMap) -> Result<Uuid, AppError> {
    let value = headers
        .get(ACTOR_HEADER)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| {
            AppError::unauthorized(
                "backoffice_actor_required",
                "Backoffice requests require x-nvbes-actor-principal-id.",
            )
        })?;
    Uuid::parse_str(value).map_err(|_| {
        AppError::bad_request(
            "invalid_backoffice_actor",
            "Backoffice actor principal ID must be a UUID.",
        )
    })
}
