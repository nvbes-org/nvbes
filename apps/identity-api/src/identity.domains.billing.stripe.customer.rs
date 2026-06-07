use super::super::types::BillingStateRecord;
use crate::http::error::AppError;
use nvbes_billing::stripe::StripeCustomer;
use nvbes_core::config::AppConfig;

pub async fn create_stripe_customer(
    config: &AppConfig,
    record: &BillingStateRecord,
) -> Result<StripeCustomer, AppError> {
    let response =
        super::http::stripe_post_form(config, "/v1/customers", build_customer_fields(record))
            .await?;

    let id = response
        .get("id")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            AppError::internal(
                "stripe_response_invalid",
                "Stripe customer response is missing id.",
            )
        })?;

    Ok(StripeCustomer { id: id.to_owned() })
}

pub fn build_customer_fields(record: &BillingStateRecord) -> Vec<(String, String)> {
    vec![
        ("email".to_string(), record.owner_email.clone()),
        ("name".to_string(), record.workspace_name.clone()),
        (
            "metadata[workspace_id]".to_string(),
            record.workspace_id.to_string(),
        ),
        (
            "metadata[owner_principal_id]".to_string(),
            record.owner_principal_id.to_string(),
        ),
    ]
}
