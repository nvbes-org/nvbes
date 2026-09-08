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

pub struct CreateProviderCheckoutInput<'a> {
    pub workspace_id: Uuid,
    pub record: &'a BillingStateRecord,
    pub target_plan: &'a PlanRecord,
    pub checkout_country: Option<&'a str>,
    pub checkout_amount_minor: i64,
    pub success_url: &'a str,
    pub cancel_url: &'a str,
    pub provider: ProviderCode,
    pub fraud_metadata: &'a [(String, String)],
}

pub async fn create_provider_checkout(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    config: &AppConfig,
    input: CreateProviderCheckoutInput<'_>,
) -> Result<CheckoutProviderResult, CheckoutProviderError> {
    match input.provider {
        ProviderCode::Stripe => create_stripe_checkout(tx, config, &input).await,
        ProviderCode::Mollie => create_mollie_checkout(tx, config, &input).await,
        ProviderCode::Cb => Err(CheckoutProviderError::ProviderNotImplemented),
    }
}

async fn create_stripe_checkout(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    config: &AppConfig,
    input: &CreateProviderCheckoutInput<'_>,
) -> Result<CheckoutProviderResult, CheckoutProviderError> {
    let mapping =
        db::fetch_active_price_mapping_tx(tx, input.target_plan.plan_id, input.checkout_country)
            .await?
            .ok_or(CheckoutProviderError::MissingProviderPriceMapping)?;
    let customer_id = match provider_customer_id_for(input.record, ProviderCode::Stripe) {
        Some(customer_id) => customer_id,
        None => {
            let customer = stripe::create_stripe_customer(config, input.record).await?;
            db::upsert_provider_customer_tx(
                tx,
                input.workspace_id,
                ProviderCode::Stripe,
                &customer.id,
            )
            .await?;
            customer.id
        }
    };
    let session = stripe::create_stripe_checkout_session(
        config,
        stripe::StripeCheckoutSessionParams {
            customer_id: &customer_id,
            owner_principal_id: input.record.owner_principal_id,
            workspace_id: input.workspace_id,
            plan_code: &input.target_plan.code,
            stripe_price_id: &mapping.stripe_price_id,
            success_url: input.success_url,
            cancel_url: input.cancel_url,
            fraud_metadata: input.fraud_metadata,
        },
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

async fn create_mollie_checkout(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    config: &AppConfig,
    input: &CreateProviderCheckoutInput<'_>,
) -> Result<CheckoutProviderResult, CheckoutProviderError> {
    let provider_customer_id = match provider_customer_id_for(input.record, ProviderCode::Mollie) {
        Some(provider_customer_id) => provider_customer_id,
        None => {
            let customer = mollie::create_mollie_customer(
                config,
                &ProviderCustomerInput {
                    tenant_id: input.record.workspace_id.to_string(),
                    email: Some(input.record.owner_email.clone()),
                    name: Some(input.record.workspace_name.clone()),
                },
            )
            .await?;
            customer.provider_customer_id
        }
    };
    db::upsert_provider_customer_tx(
        tx,
        input.workspace_id,
        ProviderCode::Mollie,
        &provider_customer_id,
    )
    .await?;
    let payment = mollie::create_mollie_payment(
        config,
        &ProviderCheckoutInput {
            tenant_id: input.record.workspace_id.to_string(),
            provider_customer_id: provider_customer_id.clone(),
            plan_code: Some(input.target_plan.code.clone()),
            amount_minor: input.checkout_amount_minor,
            currency: "EUR".to_string(),
            success_url: input.success_url.to_string(),
            cancel_url: input.cancel_url.to_string(),
            webhook_url: Some(format!(
                "{}/webhooks/mollie",
                config.billing_api_base_url().trim_end_matches('/')
            )),
            fraud_metadata: input.fraud_metadata.to_vec(),
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
