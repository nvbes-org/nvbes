use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::http::error::AppError;

pub use nvbes_billing::models::{BillingStateRecord, PlanRecord, StripePriceMapping};

pub async fn fetch_billing_state_tx(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
) -> Result<BillingStateRecord, AppError> {
    let record = nvbes_billing::db::fetch_billing_state_tx(tx, workspace_id)
        .await?
        .ok_or_else(|| AppError::not_found("workspace_not_found", "Workspace not found."))?;
    Ok(record)
}

pub async fn fetch_plan_by_code_tx(
    tx: &mut Transaction<'_, Postgres>,
    code: &str,
) -> Result<PlanRecord, AppError> {
    let record = nvbes_billing::db::fetch_plan_by_code_tx(tx, code)
        .await?
        .ok_or_else(|| AppError::not_found("plan_not_found", "Plan not found."))?;
    Ok(record)
}

pub async fn fetch_active_price_mapping_tx(
    tx: &mut Transaction<'_, Postgres>,
    plan_id: Uuid,
    country_code: Option<&str>,
) -> Result<StripePriceMapping, AppError> {
    let record = nvbes_billing::db::fetch_active_price_mapping_tx(tx, plan_id, country_code)
        .await?
        .ok_or_else(|| {
            AppError::conflict(
                "missing_stripe_price_mapping",
                "No active Stripe price mapping exists for this plan.",
            )
        })?;
    Ok(record)
}

pub async fn upsert_provider_customer_tx(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    provider: &str,
    customer_id: &str,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO billing_accounts (
          workspace_id,
          provider,
          stripe_customer_id
        )
        VALUES ($1, $2::billing_provider, $3)
        ON CONFLICT (workspace_id) DO UPDATE
        SET provider = EXCLUDED.provider,
            stripe_customer_id = EXCLUDED.stripe_customer_id,
            updated_at = NOW()
        "#,
    )
    .bind(workspace_id)
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

pub async fn plan_id_for_stripe_price_tx(
    tx: &mut Transaction<'_, Postgres>,
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

pub async fn plan_id_by_code_tx(
    tx: &mut Transaction<'_, Postgres>,
    code: &str,
) -> Result<Uuid, AppError> {
    let plan_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM plans WHERE code = $1")
        .bind(code)
        .fetch_optional(&mut **tx)
        .await?;

    plan_id.ok_or_else(|| AppError::not_found("plan_not_found", "Plan not found."))
}

pub async fn workspace_id_for_customer_tx(
    tx: &mut Transaction<'_, Postgres>,
    customer_id: &str,
) -> Result<Uuid, AppError> {
    let workspace_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT workspace_id
        FROM billing_accounts
        WHERE stripe_customer_id = $1
        UNION
        SELECT workspace_id
        FROM subscriptions
        WHERE billing_customer_id = $1
        LIMIT 1
        "#,
    )
    .bind(customer_id)
    .fetch_optional(&mut **tx)
    .await?;

    workspace_id.ok_or_else(|| {
        AppError::bad_request(
            "unknown_stripe_customer",
            "Stripe customer is not mapped to a workspace.",
        )
    })
}

pub async fn project_workspace_plan_tx(
    tx: &mut Transaction<'_, Postgres>,
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
