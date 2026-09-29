use serde_json::Value;

use crate::mollie::{MollieProviderError, mollie_post_json};
use crate::provider::{ProviderCode, ProviderCustomer, ProviderCustomerInput};
use nvbes_core::config::AppConfig;

pub async fn create_mollie_customer(
    config: &AppConfig,
    input: &ProviderCustomerInput,
) -> Result<ProviderCustomer, MollieProviderError> {
    let response = mollie_post_json(
        config,
        "/v2/customers",
        build_mollie_customer_payload(input)?,
    )
    .await?;
    customer_from_mollie_response(response)
}

pub fn build_mollie_customer_payload(
    input: &ProviderCustomerInput,
) -> Result<Value, MollieProviderError> {
    if input.tenant_id.trim().is_empty() {
        return Err(MollieProviderError::InvalidRequest {
            code: "invalid_mollie_customer",
            message: "Mollie customer tenant id is required.",
        });
    }

    Ok(serde_json::json!({
        "name": input.name.clone().unwrap_or_else(|| input.tenant_id.clone()),
        "email": input.email,
        "metadata": {
            "tenant_id": input.tenant_id,
        }
    }))
}

pub fn customer_from_mollie_response(
    response: Value,
) -> Result<ProviderCustomer, MollieProviderError> {
    let provider_customer_id =
        response
            .get("id")
            .and_then(Value::as_str)
            .ok_or(MollieProviderError::InvalidResponse {
                code: "mollie_response_invalid",
                message: "Mollie customer is missing id.",
            })?;

    Ok(ProviderCustomer {
        provider: ProviderCode::Mollie,
        provider_customer_id: provider_customer_id.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mollie_customer_payload_uses_name_email_and_tenant_metadata() {
        let payload = build_mollie_customer_payload(&ProviderCustomerInput {
            tenant_id: "tenant_1".to_string(),
            email: Some("owner@example.com".to_string()),
            name: Some("Acme".to_string()),
        })
        .expect("payload should build");

        assert_eq!(payload["name"], "Acme");
        assert_eq!(payload["email"], "owner@example.com");
        assert_eq!(payload["metadata"]["tenant_id"], "tenant_1");
    }

    #[test]
    fn mollie_customer_response_extracts_provider_customer_id() {
        let customer = customer_from_mollie_response(serde_json::json!({
            "id": "cst_123"
        }))
        .expect("customer should parse");

        assert_eq!(customer.provider, ProviderCode::Mollie);
        assert_eq!(customer.provider_customer_id, "cst_123");
    }

    #[test]
    fn mollie_customer_payload_rejects_blank_tenant_and_defaults_name() {
        let err = build_mollie_customer_payload(&ProviderCustomerInput {
            tenant_id: "   ".to_string(),
            email: None,
            name: None,
        })
        .expect_err("blank tenant");
        assert!(matches!(
            err,
            MollieProviderError::InvalidRequest {
                code: "invalid_mollie_customer",
                ..
            }
        ));

        let payload = build_mollie_customer_payload(&ProviderCustomerInput {
            tenant_id: "tenant_2".to_string(),
            email: None,
            name: None,
        })
        .expect("payload builds");
        assert_eq!(payload["name"], "tenant_2");
    }
}
