use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    config::BillingConfig,
    error::{BillingError, BillingResult},
};

pub async fn get_or_create_customer(
    db: &PgPool,
    config: &BillingConfig,
    account_id: Uuid,
    account_type: &str,
    email: Option<&str>,
) -> BillingResult<String> {
    let existing: Option<String> = sqlx::query_scalar(
        "SELECT stripe_customer_id FROM billing_customers WHERE account_id = $1 AND account_type = $2",
    )
    .bind(account_id)
    .bind(account_type)
    .fetch_optional(db)
    .await?;

    if let Some(customer_id) = existing {
        return Ok(customer_id);
    }

    // Create test customer
    let stripe_customer_id = if config.stripe_secret_key.starts_with("sk_test_dummy") {
        format!("cus_test_{}", Uuid::new_v4().simple())
    } else {
        create_stripe_test_customer(config, account_id, account_type, email).await?
    };

    let stripe_customer_id: String = sqlx::query_scalar(
        r#"
        INSERT INTO billing_customers (account_id, account_type, stripe_customer_id, email)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (account_id, account_type) DO UPDATE SET updated_at = clock_timestamp()
        RETURNING stripe_customer_id
        "#,
    )
    .bind(account_id)
    .bind(account_type)
    .bind(&stripe_customer_id)
    .bind(email)
    .fetch_one(db)
    .await?;

    Ok(stripe_customer_id)
}

async fn create_stripe_test_customer(
    config: &BillingConfig,
    account_id: Uuid,
    account_type: &str,
    email: Option<&str>,
) -> BillingResult<String> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/v1/customers",
        config.stripe_api_base_url.trim_end_matches('/')
    );

    let mut form = vec![
        ("metadata[account_id]".to_string(), account_id.to_string()),
        (
            "metadata[account_type]".to_string(),
            account_type.to_string(),
        ),
    ];
    if let Some(em) = email {
        form.push(("email".to_string(), em.to_string()));
    }

    let response = client
        .post(&url)
        .bearer_auth(&config.stripe_secret_key)
        .header(
            "Idempotency-Key",
            format!("customer_{account_type}_{account_id}"),
        )
        .form(&form)
        .send()
        .await
        .map_err(|e| BillingError::Stripe(e.to_string()))?;

    if !response.status().is_success() {
        let err_text = response.text().await.unwrap_or_default();
        return Err(BillingError::Stripe(format!(
            "failed to create Stripe customer: {err_text}"
        )));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| BillingError::Stripe(e.to_string()))?;

    json.get("id")
        .and_then(|v| v.as_str())
        .map(str::to_owned)
        .ok_or_else(|| BillingError::Stripe("missing id in Stripe customer response".into()))
}

#[cfg(all(test, feature = "database-tests"))]
#[path = "billing.customer.tests.rs"]
mod tests;
