use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct BillingAuditEventInput<'a> {
    pub workspace_id: Uuid,
    pub actor_principal_id: Option<Uuid>,
    pub action: &'a str,
    pub target_type: &'a str,
    pub target_id: Option<Uuid>,
    pub ip: Option<&'a str>,
    pub user_agent: Option<&'a str>,
    pub metadata: serde_json::Value,
}

pub async fn tenant_id_for_workspace_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: Uuid,
) -> Result<Option<Uuid>, sqlx::Error> {
    sqlx::query_scalar::<_, Uuid>("SELECT tenant_id FROM workspaces WHERE id = $1")
        .bind(workspace_id)
        .fetch_optional(tx.as_mut())
        .await
}

pub async fn plan_id_by_code_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    code: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM plans WHERE code = $1")
        .bind(code)
        .fetch_optional(tx.as_mut())
        .await
}

pub async fn project_workspace_plan_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: Uuid,
    plan_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE workspaces
        SET plan_id = $2,
            plan_code = p.code,
            updated_at = NOW()
        FROM plans p
        WHERE id = $1
          AND p.id = $2
        "#,
    )
    .bind(workspace_id)
    .bind(plan_id)
    .execute(tx.as_mut())
    .await?;

    sqlx::query(
        r#"
        UPDATE workspace_policies wp
        SET max_share_link_ttl_days = p.max_share_link_ttl_days,
            default_share_link_ttl_days = LEAST(wp.default_share_link_ttl_days, p.max_share_link_ttl_days),
            updated_at = NOW()
        FROM plans p
        WHERE wp.workspace_id = $1
          AND p.id = $2
        "#,
    )
    .bind(workspace_id)
    .bind(plan_id)
    .execute(tx.as_mut())
    .await?;

    Ok(())
}

pub async fn insert_billing_audit_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: Uuid,
    action: &'static str,
    object: serde_json::Value,
) -> Result<(), sqlx::Error> {
    insert_billing_audit_event_tx(
        tx,
        BillingAuditEventInput {
            workspace_id,
            actor_principal_id: None,
            action,
            target_type: "billing",
            target_id: Some(workspace_id),
            ip: None,
            user_agent: None,
            metadata: object,
        },
    )
    .await
}

pub async fn insert_billing_audit_event_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    input: BillingAuditEventInput<'_>,
) -> Result<(), sqlx::Error> {
    let tenant_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT tenant_id
        FROM workspaces
        WHERE id = $1
        "#,
    )
    .bind(input.workspace_id)
    .fetch_one(tx.as_mut())
    .await?;

    nvbes_audit::insert_audit_event_tx(
        tx.as_mut(),
        nvbes_audit::AuditEventInput {
            tenant_id,
            workspace_id: Some(input.workspace_id),
            actor_principal_id: input.actor_principal_id,
            action: input.action,
            target_type: input.target_type,
            target_id: input.target_id,
            ip: input.ip,
            user_agent: input.user_agent,
            metadata: input.metadata,
        },
    )
    .await
}
