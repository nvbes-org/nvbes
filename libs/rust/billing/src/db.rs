use crate::models::{BillingStateRecord, PlanRecord, ProviderPriceMapping};
use crate::pricing::regional_price_selection;
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

pub async fn fetch_billing_state_tx(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
) -> Result<Option<BillingStateRecord>, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT
          w.id AS workspace_id,
          w.name AS workspace_name,
          COALESCE(
            w.owner_user_id,
            (SELECT principal_id FROM workspace_memberships WHERE workspace_id = w.id AND role = 'owner' LIMIT 1),
            (SELECT principal_id FROM workspace_memberships WHERE workspace_id = w.id AND role = 'admin' LIMIT 1),
            '00000000-0000-0000-0000-000000000000'::uuid
          ) AS owner_principal_id,
          owner.email AS owner_email,
          w.trial_ends_at,
          p.id AS plan_id,
          p.code AS plan_code,
          p.included_storage_gb,
          p.included_users,
          p.retention_days,
          p.max_share_links,
          p.audit_level,
          p.max_share_link_ttl_days,
          COALESCE(s.status::text, 'trialing') AS subscription_status,
          s.billing_customer_id,
          s.billing_subscription_id,
          s.current_period_start,
          s.current_period_end,
          COALESCE(provider_customer.provider_customer_id, ba.stripe_customer_id) AS provider_customer_id,
          ba.stripe_customer_id,
          ba.billing_email,
          ba.country::text AS country,
          ba.customer_type::text AS customer_type,
          ba.vat_number,
          ba.tax_exempt_status,
          COALESCE(qu.used_storage_bytes, 0)::bigint AS used_storage_bytes,
          COALESCE(qu.bandwidth_out_bytes_month, 0)::bigint AS bandwidth_out_bytes_month,
          COALESCE(member_counts.active_user_count, 0)::bigint AS active_user_count
        FROM workspaces w
        INNER JOIN users owner ON owner.principal_id = COALESCE(
          w.owner_user_id,
          (SELECT principal_id FROM workspace_memberships WHERE workspace_id = w.id AND role = 'owner' LIMIT 1),
          (SELECT principal_id FROM workspace_memberships WHERE workspace_id = w.id AND role = 'admin' LIMIT 1)
        )
        INNER JOIN plans p ON p.id = COALESCE(
          w.plan_id,
          (SELECT id FROM plans WHERE code = w.plan_code LIMIT 1)
        )
        LEFT JOIN subscriptions s ON s.workspace_id = w.id
        LEFT JOIN billing_accounts ba ON ba.workspace_id = w.id
        LEFT JOIN LATERAL (
          SELECT pc.provider_customer_id
          FROM billing_provider_customers pc
          WHERE pc.billing_account_id = ba.id
            AND pc.provider = ba.provider
            AND pc.status = 'active'
          ORDER BY pc.updated_at DESC
          LIMIT 1
        ) provider_customer ON TRUE
        LEFT JOIN quota_usage qu ON qu.workspace_id = w.id
        LEFT JOIN LATERAL (
          SELECT COUNT(*)::bigint AS active_user_count
          FROM workspace_memberships wm
          WHERE wm.workspace_id = w.id
            AND wm.status = 'active'
        ) member_counts ON TRUE
        WHERE w.id = $1
        "#,
    )
    .bind(workspace_id)
    .fetch_optional(&mut **tx)
    .await?;

    Ok(row.map(|row| BillingStateRecord {
        workspace_id: row.get("workspace_id"),
        workspace_name: row.get("workspace_name"),
        owner_principal_id: row.get("owner_principal_id"),
        owner_email: row.get("owner_email"),
        trial_ends_at: row.get("trial_ends_at"),
        plan_id: row.get("plan_id"),
        plan_code: row.get("plan_code"),
        included_storage_gb: row.get("included_storage_gb"),
        included_users: row.get("included_users"),
        retention_days: row.get("retention_days"),
        max_share_links: row.get("max_share_links"),
        audit_level: row.get("audit_level"),
        max_share_link_ttl_days: row.get("max_share_link_ttl_days"),
        subscription_status: row.get("subscription_status"),
        billing_customer_id: row.get("billing_customer_id"),
        billing_subscription_id: row.get("billing_subscription_id"),
        current_period_start: row.get("current_period_start"),
        current_period_end: row.get("current_period_end"),
        provider_customer_id: row.get("provider_customer_id"),
        stripe_customer_id: row.get("stripe_customer_id"),
        billing_email: row.get("billing_email"),
        country: row.get("country"),
        customer_type: row
            .get::<Option<String>, _>("customer_type")
            .unwrap_or_else(|| "b2b".to_string()),
        vat_number: row.get("vat_number"),
        tax_exempt_status: row.get("tax_exempt_status"),
        used_storage_bytes: row.get("used_storage_bytes"),
        bandwidth_out_bytes_month: row.get("bandwidth_out_bytes_month"),
        active_user_count: row.get("active_user_count"),
    }))
}

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
