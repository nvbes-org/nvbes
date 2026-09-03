use crate::models::{PlanRecord, ProviderPriceMapping};
use crate::pricing::regional_price_selection;
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

pub async fn fetch_plan_by_code_tx(
    tx: &mut Transaction<'_, Postgres>,
    code: &str,
) -> Result<Option<PlanRecord>, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT id, code
        FROM plans
        WHERE code = $1
        "#,
    )
    .bind(code)
    .fetch_optional(&mut **tx)
    .await?;

    Ok(row.map(|row| PlanRecord {
        plan_id: row.get("id"),
        code: row.get("code"),
    }))
}

pub async fn fetch_active_price_mapping_tx(
    tx: &mut Transaction<'_, Postgres>,
    plan_id: Uuid,
    country_code: Option<&str>,
) -> Result<Option<ProviderPriceMapping>, sqlx::Error> {
    let selection = regional_price_selection(country_code);
    let provider_row = sqlx::query(
        r#"
        SELECT
          provider_product_id,
          provider_price_id,
          provider_product_id AS stripe_product_id,
          provider_price_id AS stripe_price_id,
          country_code::text AS country_code,
          pricing_region,
          currency::text AS currency,
          amount_minor
        FROM billing_provider_price_mappings
        WHERE provider = 'stripe'
          AND legacy_plan_id = $1
          AND status = 'active'
          AND (
            country_code = $2::char(2)
            OR (
              country_code IS NULL
              AND pricing_region = $3
            )
            OR (
              country_code IS NULL
              AND pricing_region IS NULL
            )
          )
        ORDER BY
          CASE
            WHEN country_code = $2::char(2) THEN 0
            WHEN country_code IS NULL AND pricing_region = $3 THEN 1
            ELSE 2
          END,
          created_at DESC
        LIMIT 1
        "#,
    )
    .bind(plan_id)
    .bind(selection.country_code.as_deref())
    .bind(selection.pricing_region.as_deref())
    .fetch_optional(&mut **tx)
    .await?;

    if let Some(row) = provider_row {
        return Ok(Some(ProviderPriceMapping {
            provider_product_id: row.get("provider_product_id"),
            provider_price_id: row.get("provider_price_id"),
            stripe_product_id: row.get("stripe_product_id"),
            stripe_price_id: row.get("stripe_price_id"),
            country_code: row.get("country_code"),
            pricing_region: row.get("pricing_region"),
            currency: row.get("currency"),
            amount_minor: row.get("amount_minor"),
        }));
    }

    let legacy_row = sqlx::query(
        r#"
        SELECT
          stripe_product_id AS provider_product_id,
          stripe_price_id AS provider_price_id,
          stripe_product_id,
          stripe_price_id,
          country_code::text AS country_code,
          pricing_region,
          currency::text AS currency,
          amount_minor
        FROM stripe_price_mappings
        WHERE plan_id = $1
          AND meter = 'subscription'
          AND status = 'active'
          AND valid_from <= NOW()
          AND (valid_until IS NULL OR valid_until > NOW())
          AND (
            country_code = $2::char(2)
            OR (
              country_code IS NULL
              AND pricing_region = $3
            )
            OR (
              country_code IS NULL
              AND pricing_region IS NULL
            )
          )
        ORDER BY
          CASE
            WHEN country_code = $2::char(2) THEN 0
            WHEN country_code IS NULL AND pricing_region = $3 THEN 1
            ELSE 2
          END,
          valid_from DESC
        LIMIT 1
        "#,
    )
    .bind(plan_id)
    .bind(selection.country_code.as_deref())
    .bind(selection.pricing_region.as_deref())
    .fetch_optional(&mut **tx)
    .await?;

    Ok(legacy_row.map(|row| ProviderPriceMapping {
        provider_product_id: row.get("provider_product_id"),
        provider_price_id: row.get("provider_price_id"),
        stripe_product_id: row.get("stripe_product_id"),
        stripe_price_id: row.get("stripe_price_id"),
        country_code: row.get("country_code"),
        pricing_region: row.get("pricing_region"),
        currency: row.get("currency"),
        amount_minor: row.get("amount_minor"),
    }))
}

pub async fn plan_id_for_provider_price_tx(
    tx: &mut Transaction<'_, Postgres>,
    provider: &str,
    provider_price_id: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    if let Some(plan_id) = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT legacy_plan_id
        FROM billing_provider_price_mappings
        WHERE provider = $1::billing_provider
          AND provider_price_id = $2
          AND status = 'active'
          AND legacy_plan_id IS NOT NULL
        ORDER BY created_at DESC
        LIMIT 1
        "#,
    )
    .bind(provider)
    .bind(provider_price_id)
    .fetch_optional(&mut **tx)
    .await?
    {
        return Ok(Some(plan_id));
    }

    if provider != "stripe" {
        return Ok(None);
    }

    sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT plan_id
        FROM stripe_price_mappings
        WHERE stripe_price_id = $1
          AND status = 'active'
        ORDER BY valid_from DESC
        LIMIT 1
        "#,
    )
    .bind(provider_price_id)
    .fetch_optional(&mut **tx)
    .await
}

pub async fn workspace_id_for_provider_customer_tx(
    tx: &mut Transaction<'_, Postgres>,
    provider: &str,
    provider_customer_id: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT ba.workspace_id
        FROM billing_provider_customers pc
        INNER JOIN billing_accounts ba ON ba.id = pc.billing_account_id
        WHERE pc.provider = $1::billing_provider
          AND pc.provider_customer_id = $2
          AND pc.status = 'active'
        UNION
        SELECT workspace_id
        FROM billing_accounts
        WHERE provider = $1::billing_provider
          AND stripe_customer_id = $2
        UNION
        SELECT workspace_id
        FROM subscriptions
        WHERE billing_provider = $1::billing_provider
          AND billing_customer_id = $2
        LIMIT 1
        "#,
    )
    .bind(provider)
    .bind(provider_customer_id)
    .fetch_optional(&mut **tx)
    .await
}

pub async fn ensure_plan_seeded_tx(tx: &mut Transaction<'_, Postgres>) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO plans (code, name, max_share_link_ttl_days)
        VALUES
          ('trial', 'Trial', 30),
          ('solo_pro', 'Solo Pro', 30)
        ON CONFLICT (code) DO NOTHING
        "#,
    )
    .execute(tx.as_mut())
    .await?;
    Ok(())
}
