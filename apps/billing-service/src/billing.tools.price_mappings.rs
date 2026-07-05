use anyhow::{Context, bail};

const REQUIRED_PROVIDER_PRICE_PLAN_CODES: [&str; 3] = ["solo_pro", "team", "team_plus"];

pub async fn check_active_provider_price_mappings(pool: &sqlx::PgPool) -> anyhow::Result<()> {
    for plan_code in REQUIRED_PROVIDER_PRICE_PLAN_CODES {
        let plan_id: Option<uuid::Uuid> = sqlx::query_scalar(
            r#"
            SELECT id
            FROM plans
            WHERE code = $1
            "#,
        )
        .bind(plan_code)
        .fetch_optional(pool)
        .await
        .context("Failed to read Billing plan records.")?;

        let Some(plan_id) = plan_id else {
            bail!("Billing plan {plan_code} is missing.");
        };

        let mapping: Option<(String, String, String)> = sqlx::query_as(
            r#"
            SELECT provider::text, provider_product_id, provider_price_id
            FROM billing_provider_price_mappings
            WHERE legacy_plan_id = $1
              AND status = 'active'
            ORDER BY updated_at DESC, created_at DESC
            LIMIT 1
            "#,
        )
        .bind(plan_id)
        .fetch_optional(pool)
        .await
        .context("Failed to read Billing provider price mappings.")?;

        let Some((provider, provider_product_id, provider_price_id)) = mapping else {
            bail!("Missing active provider price mapping for Billing plan {plan_code}.");
        };

        if provider == "stripe"
            && (!provider_product_id.starts_with("prod_")
                || !provider_price_id.starts_with("price_"))
        {
            bail!(
                "Provider price mapping for Billing plan {plan_code} does not look like a real Stripe test mapping."
            );
        }

        println!(
            "{plan_code}: provider={provider} product={provider_product_id} price={provider_price_id}"
        );
    }

    Ok(())
}
