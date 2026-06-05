use serde_json::Value;
use sqlx::{PgConnection, Postgres};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AuditEventInput<'a> {
    pub tenant_id: Uuid,
    pub workspace_id: Option<Uuid>,
    pub actor_principal_id: Option<Uuid>,
    pub action: &'a str,
    pub target_type: &'a str,
    pub target_id: Option<Uuid>,
    pub ip: Option<&'a str>,
    pub user_agent: Option<&'a str>,
    pub metadata: Value,
}

/// Insère un événement d'audit en utilisant un pool de connexion.
/// Le trigger SQL se charge de calculer le chaînage de hash pour l'immuabilité.
pub async fn insert_audit_event_pool<'a>(
    executor: &'a sqlx::Pool<Postgres>,
    input: AuditEventInput<'a>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO audit_events (
          tenant_id,
          workspace_id,
          actor_principal_id,
          action,
          target_type,
          target_id,
          ip,
          user_agent,
          metadata
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7::inet, $8, $9)
        "#,
    )
    .bind(input.tenant_id)
    .bind(input.workspace_id)
    .bind(input.actor_principal_id)
    .bind(input.action)
    .bind(input.target_type)
    .bind(input.target_id)
    .bind(input.ip)
    .bind(input.user_agent)
    .bind(sqlx::types::Json(input.metadata))
    .execute(executor)
    .await?;

    Ok(())
}

/// Insère un événement d'audit dans une transaction existante.
pub async fn insert_audit_event_tx<'a>(
    executor: &'a mut PgConnection,
    input: AuditEventInput<'a>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO audit_events (
          tenant_id,
          workspace_id,
          actor_principal_id,
          action,
          target_type,
          target_id,
          ip,
          user_agent,
          metadata
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7::inet, $8, $9)
        "#,
    )
    .bind(input.tenant_id)
    .bind(input.workspace_id)
    .bind(input.actor_principal_id)
    .bind(input.action)
    .bind(input.target_type)
    .bind(input.target_id)
    .bind(input.ip)
    .bind(input.user_agent)
    .bind(sqlx::types::Json(input.metadata))
    .execute(executor)
    .await?;

    Ok(())
}
