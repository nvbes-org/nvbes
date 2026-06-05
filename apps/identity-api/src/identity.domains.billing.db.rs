use super::types::{AuditEventInput, BillingStateRecord, PlanRecord, StripePriceMapping};
use crate::http::error::AppError;
use sqlx::PgPool;
use uuid::Uuid;

pub async fn fetch_billing_state_pool(
    db: &PgPool,
    workspace_id: Uuid,
) -> Result<BillingStateRecord, AppError> {
    let mut tx = db.begin().await?;
    let record = fetch_billing_state_tx(&mut tx, workspace_id).await?;
    tx.commit().await?;
    Ok(record)
}

pub async fn fetch_billing_state_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: Uuid,
) -> Result<BillingStateRecord, AppError> {
    let record = nvbes_billing::db::fetch_billing_state_tx(tx, workspace_id)
        .await?
        .ok_or_else(|| AppError::not_found("workspace_not_found", "Workspace not found."))?;
    Ok(record)
}

pub async fn fetch_plan_by_code_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    code: &str,
) -> Result<PlanRecord, AppError> {
    let record = nvbes_billing::db::fetch_plan_by_code_tx(tx, code)
        .await?
        .ok_or_else(|| AppError::not_found("plan_not_found", "Plan not found."))?;
    Ok(record)
}

pub async fn fetch_active_price_mapping_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    plan_id: Uuid,
) -> Result<StripePriceMapping, AppError> {
    let record = nvbes_billing::db::fetch_active_price_mapping_tx(tx, plan_id)
        .await?
        .ok_or_else(|| {
            AppError::conflict(
                "missing_stripe_price_mapping",
                "No active Stripe price mapping exists for this plan.",
            )
        })?;
    Ok(record)
}

pub async fn upsert_billing_customer_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: Uuid,
    customer_id: &str,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO billing_accounts (
          workspace_id,
          provider,
          stripe_customer_id
        )
        VALUES ($1, 'stripe', $2)
        ON CONFLICT (workspace_id) DO UPDATE
        SET stripe_customer_id = EXCLUDED.stripe_customer_id,
            updated_at = NOW()
        "#,
    )
    .bind(workspace_id)
    .bind(customer_id)
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        r#"
        UPDATE subscriptions
        SET billing_customer_id = $2,
            updated_at = NOW()
        WHERE workspace_id = $1
        "#,
    )
    .bind(workspace_id)
    .bind(customer_id)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub(crate) async fn insert_audit_event(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    input: AuditEventInput<'_>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO audit_events (
          workspace_id,
          actor_user_id,
          action,
          target_type,
          target_id,
          ip,
          user_agent,
          metadata
        )
        VALUES ($1, $2, $3, $4, $5, $6::inet, $7, $8)
        "#,
    )
    .bind(input.workspace_id)
    .bind(input.actor_user_id)
    .bind(input.action)
    .bind(input.target_type)
    .bind(input.target_id)
    .bind(input.ip)
    .bind(input.user_agent)
    .bind(sqlx::types::Json(input.metadata))
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub async fn plan_id_for_stripe_price(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    stripe_price_id: &str,
) -> Result<Uuid, AppError> {
    let plan_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT plan_id
        FROM stripe_price_mappings
        WHERE stripe_price_id = $1
          AND status = 'active'
        ORDER BY valid_from DESC
        LIMIT 1
        "#,
    )
    .bind(stripe_price_id)
    .fetch_optional(&mut **tx)
    .await?;

    plan_id.ok_or_else(|| {
        AppError::bad_request(
            "unknown_stripe_price",
            "Stripe price is not mapped to a nvbes plan.",
        )
    })
}

pub async fn plan_id_by_code(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    code: &str,
) -> Result<Uuid, AppError> {
    let plan_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM plans WHERE code = $1")
        .bind(code)
        .fetch_optional(&mut **tx)
        .await?;

    plan_id.ok_or_else(|| AppError::not_found("plan_not_found", "Plan not found."))
}

pub async fn workspace_id_for_subscription(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    subscription_id: &str,
) -> Result<Uuid, AppError> {
    let workspace_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT workspace_id
        FROM subscriptions
        WHERE billing_subscription_id = $1
        "#,
    )
    .bind(subscription_id)
    .fetch_optional(&mut **tx)
    .await?;

    workspace_id.ok_or_else(|| {
        AppError::bad_request(
            "unknown_stripe_subscription",
            "Stripe subscription is not mapped to a workspace.",
        )
    })
}

pub async fn project_workspace_plan(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: Uuid,
    plan_id: Uuid,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE workspaces
        SET plan_id = $2,
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(workspace_id)
    .bind(plan_id)
    .execute(&mut **tx)
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
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub async fn insert_billing_audit(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: Uuid,
    action: &'static str,
    object: &serde_json::Value,
) -> Result<(), AppError> {
    insert_audit_event(
        tx,
        AuditEventInput {
            workspace_id,
            actor_user_id: None,
            action,
            target_type: "billing",
            target_id: Some(workspace_id),
            ip: None,
            user_agent: None,
            metadata: object.clone(),
        },
    )
    .await
}

pub async fn ensure_plan_seeded(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO plans (code, name, max_share_link_ttl_days)
        VALUES
          ('trial', 'Trial', 30),
          ('solo_pro', 'Solo Pro', 30)
        ON CONFLICT (code) DO NOTHING
        "#,
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}
