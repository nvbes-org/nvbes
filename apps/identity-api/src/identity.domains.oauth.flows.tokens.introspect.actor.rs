use crate::http::error::AppError;
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
        SELECT
          sa.workspace_id,
          w.organization_id,
          w.tenant_id,
          wm.role::text AS role
        FROM service_accounts sa
        INNER JOIN workspaces w ON w.id = sa.workspace_id
        LEFT JOIN workspace_memberships wm
          ON wm.workspace_id = sa.workspace_id
         AND wm.principal_id = sa.principal_id
         AND wm.status = 'active'
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

    Ok((
        Some("service_account".to_string()),
        row.get("role"),
        row.get("workspace_id"),
        row.get("organization_id"),
        row.get("tenant_id"),
    ))
}
