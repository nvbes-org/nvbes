use crate::hex_encode;
use crate::provider::{ProviderCode, ProviderPaymentMethod};
use sha2::{Digest, Sha256};

pub async fn upsert_provider_payment_method_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    provider: ProviderCode,
    provider_customer_id: &str,
    method: &ProviderPaymentMethod,
) -> Result<(), sqlx::Error> {
    let Some(provider_payment_method_id) = method.provider_payment_method_id.as_deref() else {
        return Ok(());
    };
    let fingerprint_hash = method
        .fingerprint
        .as_deref()
        .map(|fingerprint| hex_encode(&Sha256::digest(fingerprint.as_bytes())));
    let display_label = payment_method_display_label(method);

    sqlx::query(
        r#"
        WITH provider_customer AS (
          SELECT pc.id AS provider_customer_row_id,
                 pc.tenant_id,
                 pc.billing_account_id
          FROM billing_provider_customers pc
          WHERE pc.provider = $1::billing_provider
            AND pc.provider_customer_id = $2
            AND pc.status = 'active'
          LIMIT 1
        ),
        existing_method AS (
          SELECT pm.id
          FROM billing_payment_methods pm
          JOIN provider_customer pc ON pc.billing_account_id = pm.billing_account_id
          WHERE $10::text IS NOT NULL
            AND pm.fingerprint_hash = $10
          LIMIT 1
        ),
        inserted_method AS (
          INSERT INTO billing_payment_methods (
            tenant_id,
            billing_account_id,
            method_type,
            display_label,
            brand,
            last4,
            exp_month,
            exp_year,
            funding,
            issuer_country,
            fingerprint_hash,
            status,
            is_primary
          )
          SELECT pc.tenant_id,
                 pc.billing_account_id,
                 $3,
                 $4,
                 $5,
                 $6,
                 $7,
                 $8,
                 $9,
                 $11::char(2),
                 $10,
                 'active',
                 TRUE
          FROM provider_customer pc
          WHERE NOT EXISTS (SELECT 1 FROM existing_method)
          RETURNING id
        ),
        selected_method AS (
          SELECT id FROM existing_method
          UNION ALL
          SELECT id FROM inserted_method
          LIMIT 1
        )
        INSERT INTO billing_provider_payment_methods (
          tenant_id,
          payment_method_id,
          provider_customer_id,
          provider,
          provider_payment_method_id,
          mandate_id,
          mandate_status,
          status,
          reusable
        )
        SELECT pc.tenant_id,
               sm.id,
               pc.provider_customer_row_id,
               $1::billing_provider,
               $12,
               $13,
               $14,
               'active',
               $15
        FROM provider_customer pc
        CROSS JOIN selected_method sm
        ON CONFLICT (provider, provider_payment_method_id) DO UPDATE
        SET payment_method_id = EXCLUDED.payment_method_id,
            provider_customer_id = EXCLUDED.provider_customer_id,
            mandate_id = EXCLUDED.mandate_id,
            mandate_status = EXCLUDED.mandate_status,
            status = 'active',
            reusable = EXCLUDED.reusable,
            updated_at = NOW()
        "#,
    )
    .bind(provider.as_str())
    .bind(provider_customer_id)
    .bind(&method.method_type)
    .bind(display_label)
    .bind(&method.brand)
    .bind(&method.last4)
    .bind(method.exp_month)
    .bind(method.exp_year)
    .bind(&method.funding)
    .bind(&fingerprint_hash)
    .bind(&method.issuer_country)
    .bind(provider_payment_method_id)
    .bind(&method.mandate_id)
    .bind(&method.mandate_status)
    .bind(method.reusable)
    .execute(tx.as_mut())
    .await?;

    Ok(())
}

fn payment_method_display_label(method: &ProviderPaymentMethod) -> Option<String> {
    match (&method.brand, &method.last4) {
        (Some(brand), Some(last4)) => Some(format!("{brand} **** {last4}")),
        (Some(brand), None) => Some(brand.clone()),
        (None, Some(last4)) => Some(format!("Card **** {last4}")),
        (None, None) => None,
    }
}
