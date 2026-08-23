use nvbes_billing::{StripeWebhookEvent, provider::ProviderPayment};
use nvbes_email::EmailTemplate;
use serde_json::Value;

pub(crate) fn stripe_event_template(event: &StripeWebhookEvent) -> Option<EmailTemplate> {
    match event.event_type.as_str() {
        "invoice.payment_failed" => Some(payment_failed_template(&event.data_object)),
        "invoice.payment_succeeded" => Some(receipt_template(&event.data_object)),
        _ => None,
    }
}

pub(crate) fn provider_payment_template(payment: &ProviderPayment) -> Option<EmailTemplate> {
    let customer_name = None;
    let currency = payment.currency.to_uppercase();
    let provider_name = Some(payment.provider.as_str().to_string());
    match payment.status.as_str() {
        "failed" => Some(EmailTemplate::BillingPaymentFailureV1 {
            customer_name,
            amount_minor: payment.amount_minor,
            currency,
            billing_portal_url: None,
            invoice_url: None,
            provider_name,
        }),
        "paid" | "succeeded" => Some(EmailTemplate::BillingReceiptV1 {
            customer_name,
            amount_minor: payment.amount_minor,
            currency,
            invoice_url: None,
            provider_name,
        }),
        _ => None,
    }
}

pub(crate) fn billing_email_idempotency_key(
    provider_event_id: &str,
    template: &EmailTemplate,
) -> String {
    format!(
        "billing-email:{}:{provider_event_id}",
        template.name_and_version().0
    )
}

fn payment_failed_template(object: &Value) -> EmailTemplate {
    EmailTemplate::BillingPaymentFailureV1 {
        customer_name: None,
        amount_minor: object
            .get("amount_due")
            .and_then(Value::as_i64)
            .unwrap_or(0),
        currency: invoice_currency(object),
        billing_portal_url: None,
        invoice_url: optional_url(object, "hosted_invoice_url"),
        provider_name: Some("stripe".to_string()),
    }
}

fn receipt_template(object: &Value) -> EmailTemplate {
    EmailTemplate::BillingReceiptV1 {
        customer_name: None,
        amount_minor: object
            .get("amount_paid")
            .and_then(Value::as_i64)
            .unwrap_or(0),
        currency: invoice_currency(object),
        invoice_url: optional_url(object, "hosted_invoice_url"),
        provider_name: Some("stripe".to_string()),
    }
}

fn invoice_currency(object: &Value) -> String {
    object
        .get("currency")
        .and_then(Value::as_str)
        .unwrap_or("eur")
        .to_uppercase()
}

fn optional_url(object: &Value, field: &str) -> Option<String> {
    object
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{billing_email_idempotency_key, payment_failed_template};

    #[test]
    fn billing_payload_is_a_closed_template_and_preserves_provider_event_identity() {
        let template = payment_failed_template(&json!({
            "amount_due": 4680,
            "currency": "eur",
            "hosted_invoice_url": "https://billing.example/invoice/1"
        }));
        assert_eq!(
            billing_email_idempotency_key("evt_1", &template),
            "billing-email:billing_payment_failure:evt_1"
        );
    }
}
