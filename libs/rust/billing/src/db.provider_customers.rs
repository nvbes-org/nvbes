use crate::provider::ProviderCode;
use sqlx::Row;
use uuid::Uuid;

pub async fn upsert_provider_customer_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: Uuid,
    provider: ProviderCode,
    customer_id: &str,
) -> Result<(), sqlx::Error> {
    upsert_provider_customer_with_legacy_subscription_tx(
        tx,
        workspace_id,
        provider,
        customer_id,
        true,
    )
    .await
}

pub async fn upsert_provider_customer_mapping_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: Uuid,
    provider: ProviderCode,
    customer_id: &str,
) -> Result<(), sqlx::Error> {
    upsert_provider_customer_with_legacy_subscription_tx(
        tx,
        workspace_id,
        provider,
        customer_id,
        false,
    )
    .await
}

async fn upsert_provider_customer_with_legacy_subscription_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: Uuid,
    provider: ProviderCode,
    customer_id: &str,
    update_legacy_subscription: bool,
) -> Result<(), sqlx::Error> {
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
            stripe_customer_id = CASE
              WHEN EXCLUDED.provider = 'stripe'::billing_provider THEN EXCLUDED.stripe_customer_id
              ELSE billing_accounts.stripe_customer_id
            END,
            updated_at = NOW()
        RETURNING id, tenant_id
        "#,
    )
    .bind(workspace_id)
    .bind(provider.as_str())
    .bind(if provider == ProviderCode::Stripe {
        Some(customer_id)
    } else {
        None
    })
    .fetch_one(tx.as_mut())
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
    .bind(provider.as_str())
    .bind(customer_id)
    .execute(tx.as_mut())
    .await?;

    if update_legacy_subscription {
        sqlx::query(
            r#"
            UPDATE subscriptions
            SET billing_provider = $2::billing_provider,
                billing_customer_id = $3,
                updated_at = NOW()
            WHERE workspace_id = $1
            "#,
        )
        .bind(workspace_id)
        .bind(provider.as_str())
        .bind(customer_id)
        .execute(tx.as_mut())
        .await?;
    }

    Ok(())
}
