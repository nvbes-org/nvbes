#[path = "identity.domains.billing.db.provider_events.rs"]
mod provider_events;
#[path = "identity.domains.billing.db.provider_routing.rs"]
mod provider_routing;

pub use provider_events::{mark_provider_event_replayed, record_provider_event};
pub use provider_routing::{ProviderRoutingRule, fetch_provider_routing_rule_tx};

use super::types::{AuditEventInput, BillingStateRecord, PlanRecord, ProviderPriceMapping};
use crate::http::error::AppError;
use sqlx::{PgPool, Row};
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
    country_code: Option<&str>,
) -> Result<ProviderPriceMapping, AppError> {
    nvbes_billing::db::fetch_active_price_mapping_tx(tx, plan_id, country_code)
        .await?
        .ok_or_else(|| {
            AppError::conflict(
                "missing_provider_price_mapping",
                "No active provider price mapping exists for this plan.",
            )
        })
}

pub async fn upsert_provider_customer_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: Uuid,
    provider: &str,
    customer_id: &str,
) -> Result<(), AppError> {
    let account = sqlx::query(
        r#"
        INSERT INTO billing_accounts (
          workspace_id,
          tenant_id,
          provider,
          stripe_customer_id
        )
        SELECT id, tenant_id, $2::billing_provider, $3
        FROM workspaces
        WHERE id = $1
        ON CONFLICT (workspace_id) DO UPDATE
        SET provider = EXCLUDED.provider,
            stripe_customer_id = EXCLUDED.stripe_customer_id,
            updated_at = NOW()
        RETURNING id, tenant_id
        "#,
    )
    .bind(workspace_id)
    .bind(provider)
    .bind(customer_id)
    .fetch_one(&mut **tx)
    .await?;
    let billing_account_id: Uuid = account.get("id");
    let tenant_id: Uuid = account.get("tenant_id");

    sqlx::query(
        r#"
        INSERT INTO billing_provider_customers (
          tenant_id,
          billing_account_id,
          provider,
          provider_customer_id,
          status
        )
        VALUES ($1, $2, $3::billing_provider, $4, 'active')
        ON CONFLICT (provider, provider_customer_id) DO UPDATE
        SET tenant_id = EXCLUDED.tenant_id,
            billing_account_id = EXCLUDED.billing_account_id,
            status = 'active',
            updated_at = NOW()
        "#,
    )
    .bind(tenant_id)
    .bind(billing_account_id)
    .bind(provider)
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
    let tenant_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT tenant_id
        FROM workspaces
        WHERE id = $1
        "#,
    )
    .bind(input.workspace_id)
    .fetch_one(&mut **tx)
    .await?;

    crate::domains::audit::record_event_tx(
        tx,
        crate::domains::audit::AuditRecordInput {
            tenant_id,
            workspace_id: Some(input.workspace_id),
            actor_principal_id: input.actor_user_id,
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

pub async fn plan_id_for_provider_price(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    provider: &str,
    provider_price_id: &str,
) -> Result<Uuid, AppError> {
    nvbes_billing::db::plan_id_for_provider_price_tx(tx, provider, provider_price_id)
        .await?
        .ok_or_else(|| {
            AppError::bad_request(
                "unknown_provider_price",
                "Provider price is not mapped to a nvbes plan.",
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
            "unknown_provider_subscription",
            "Provider subscription is not mapped to a workspace.",
        )
    })
}

pub async fn tenant_id_for_workspace(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: Uuid,
) -> Result<Uuid, AppError> {
    let tenant_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT tenant_id
        FROM workspaces
        WHERE id = $1
        "#,
    )
    .bind(workspace_id)
    .fetch_optional(&mut **tx)
    .await?;

    tenant_id.ok_or_else(|| AppError::not_found("workspace_not_found", "Workspace not found."))
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
