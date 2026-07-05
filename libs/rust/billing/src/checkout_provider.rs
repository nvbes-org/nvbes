use crate::checkout_routing::CheckoutProviderResult;
use crate::models::{BillingStateRecord, PlanRecord};
use crate::provider::{ProviderCheckoutInput, ProviderCode, ProviderCustomerInput};
use crate::types::{CheckoutSessionResponse, CreateCheckoutInput};
use crate::{db, mollie, provider_customer_id_for, stripe};
use nvbes_core::config::AppConfig;
use sqlx::Postgres;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum CheckoutProviderError {
    #[error("missing_provider_price_mapping")]
    MissingProviderPriceMapping,
    #[error("invalid_plan")]
    InvalidPlan,
    #[error("plan_not_found")]
    PlanNotFound,
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    Stripe(#[from] stripe::StripeProviderError),
    #[error(transparent)]
    Mollie(#[from] mollie::MollieProviderError),
    #[error("provider_not_implemented")]
    ProviderNotImplemented,
}

pub async fn fetch_paid_checkout_plan(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    input: &CreateCheckoutInput,
) -> Result<PlanRecord, CheckoutProviderError> {
    let target_plan_code =
        crate::validate_plan_code(&input.plan_code).ok_or(CheckoutProviderError::InvalidPlan)?;
    let target_plan = db::fetch_plan_by_code_tx(tx, &target_plan_code)
        .await?
        .ok_or(CheckoutProviderError::PlanNotFound)?;

    if target_plan.code == "trial" {
        return Err(CheckoutProviderError::InvalidPlan);
    }

    Ok(target_plan)
}

pub fn checkout_session_response(checkout: CheckoutProviderResult) -> CheckoutSessionResponse {
    CheckoutSessionResponse {
        provider: checkout.provider.as_str().to_string(),
        url: checkout.url,
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "Provider checkout creation keeps routing, URLs, and billing records explicit."
)]
pub async fn create_provider_checkout(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    config: &AppConfig,
    workspace_id: Uuid,
    record: &BillingStateRecord,
    target_plan: &PlanRecord,
    checkout_country: Option<&str>,
    checkout_amount_minor: i64,
    success_url: &str,
    cancel_url: &str,
    provider: ProviderCode,
    fraud_metadata: &[(String, String)],
) -> Result<CheckoutProviderResult, CheckoutProviderError> {
    match provider {
        ProviderCode::Stripe => {
            create_stripe_checkout(
                tx,
                config,
                workspace_id,
                record,
                target_plan,
                checkout_country,
                success_url,
                cancel_url,
                fraud_metadata,
            )
            .await
        }
        ProviderCode::Mollie => {
            create_mollie_checkout(
                tx,
                config,
                workspace_id,
                record,
                target_plan,
                checkout_amount_minor,
                success_url,
                cancel_url,
                fraud_metadata,
            )
            .await
        }
        ProviderCode::Cb => Err(CheckoutProviderError::ProviderNotImplemented),
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "Stripe checkout needs explicit billing record, plan, region, and redirect URLs."
)]
async fn create_stripe_checkout(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    config: &AppConfig,
    workspace_id: Uuid,
    record: &BillingStateRecord,
    target_plan: &PlanRecord,
    checkout_country: Option<&str>,
    success_url: &str,
    cancel_url: &str,
    fraud_metadata: &[(String, String)],
) -> Result<CheckoutProviderResult, CheckoutProviderError> {
    let mapping = db::fetch_active_price_mapping_tx(tx, target_plan.plan_id, checkout_country)
        .await?
        .ok_or(CheckoutProviderError::MissingProviderPriceMapping)?;
    let customer_id = match provider_customer_id_for(record, ProviderCode::Stripe) {
        Some(customer_id) => customer_id,
        None => {
            let customer = stripe::create_stripe_customer(config, record).await?;
            db::upsert_provider_customer_tx(tx, workspace_id, ProviderCode::Stripe, &customer.id)
                .await?;
            customer.id
        }
    };
    let session = stripe::create_stripe_checkout_session(
        config,
        &customer_id,
        record.owner_principal_id,
        workspace_id,
        &target_plan.code,
        &mapping.stripe_price_id,
        success_url,
        cancel_url,
        fraud_metadata,
    )
    .await?;

    Ok(CheckoutProviderResult {
        provider: ProviderCode::Stripe,
        checkout_id: session.id,
        url: session.url,
        provider_customer_id: customer_id.clone(),
        provider_product_id: Some(mapping.provider_product_id.clone()),
        provider_price_id: Some(mapping.provider_price_id.clone()),
        price_country_code: mapping.country_code,
        pricing_region: mapping.pricing_region,
        payment_id: None,
        stripe_customer_id: Some(customer_id),
        stripe_price_id: Some(mapping.stripe_price_id),
    })
}

#[expect(
    clippy::too_many_arguments,
    reason = "Mollie checkout needs explicit billing record, plan, amount, and redirect URLs."
)]
async fn create_mollie_checkout(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    config: &AppConfig,
    workspace_id: Uuid,
    record: &BillingStateRecord,
    target_plan: &PlanRecord,
    checkout_amount_minor: i64,
    success_url: &str,
    cancel_url: &str,
    fraud_metadata: &[(String, String)],
) -> Result<CheckoutProviderResult, CheckoutProviderError> {
    let provider_customer_id = match provider_customer_id_for(record, ProviderCode::Mollie) {
        Some(provider_customer_id) => provider_customer_id,
        None => {
            let customer = mollie::create_mollie_customer(
                config,
                &ProviderCustomerInput {
                    tenant_id: record.workspace_id.to_string(),
                    email: Some(record.owner_email.clone()),
                    name: Some(record.workspace_name.clone()),
                },
            )
            .await?;
            customer.provider_customer_id
        }
    };
    db::upsert_provider_customer_tx(
        tx,
        workspace_id,
        ProviderCode::Mollie,
        &provider_customer_id,
    )
    .await?;
    let payment = mollie::create_mollie_payment(
        config,
        &ProviderCheckoutInput {
            tenant_id: record.workspace_id.to_string(),
            provider_customer_id: provider_customer_id.clone(),
            plan_code: Some(target_plan.code.clone()),
            amount_minor: checkout_amount_minor,
            currency: "EUR".to_string(),
            success_url: success_url.to_string(),
            cancel_url: cancel_url.to_string(),
            webhook_url: Some(format!(
                "{}/webhooks/mollie",
                config.billing_api_base_url().trim_end_matches('/')
            )),
            fraud_metadata: fraud_metadata.to_vec(),
        },
    )
    .await?;

    Ok(CheckoutProviderResult {
        provider: ProviderCode::Mollie,
        checkout_id: payment.checkout_id.clone(),
        url: payment.url,
        provider_customer_id,
        provider_product_id: None,
        provider_price_id: None,
        price_country_code: None,
        pricing_region: None,
        payment_id: Some(payment.checkout_id),
        stripe_customer_id: None,
        stripe_price_id: None,
    })
}
