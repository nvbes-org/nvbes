use crate::{cloud_boundary::workspace_port, http::error::AppError};
use sqlx::Row;
use uuid::Uuid;

pub async fn resolve_actor_context(
    db: &sqlx::PgPool,
    actor_claim: Option<&crate::domains::auth::jwt::types::ActorClaim>,
) -> Result<
    (
        Option<String>,
        Option<String>,
        Option<Uuid>,
        Option<Uuid>,
        Option<Uuid>,
    ),
    AppError,
> {
    let Some(actor_claim) = actor_claim else {
        return Ok((None, None, None, None, None));
    };
    let actor_principal_id = Uuid::parse_str(&actor_claim.sub)
        .map_err(|_| AppError::unauthorized("invalid_grant", "The actor token is invalid."))?;

    let row = sqlx::query(
        r#"
        SELECT sa.workspace_id
        FROM service_accounts sa
        WHERE sa.principal_id = $1
        LIMIT 1
        "#,
    )
    .bind(actor_principal_id)
    .fetch_optional(db)
    .await?;

    let Some(row) = row else {
        return Ok((Some("service_account".to_string()), None, None, None, None));
    };
    let workspace_id: Option<Uuid> = row.get("workspace_id");
    let Some(workspace_id) = workspace_id else {
        return Ok((Some("service_account".to_string()), None, None, None, None));
    };

    let workspace = workspace_port::get_workspace(None, workspace_id, actor_principal_id).await?;
    let role = workspace_port::list_workspace_members(
        Some(workspace.tenant_id),
        workspace_id,
        actor_principal_id,
    )
    .await?
    .into_iter()
    .find(|member| member.principal_id == actor_principal_id && member.active)
    .map(|member| member.role);

    Ok((
        Some("service_account".to_string()),
        role,
        Some(workspace_id),
        workspace.organization_id,
        Some(workspace.tenant_id),
    ))
}
