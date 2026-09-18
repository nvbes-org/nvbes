use chrono::{Duration, Utc};
use nvbes_email::{
    EmailCategory, EmailClient, EmailCommand, EmailIdempotencyKey, EmailRecipient,
    EmailRequestContext, EmailTemplate,
};
use uuid::Uuid;

pub struct BillingReceiptNotification {
    pub account_id: Uuid,
    pub recipient_email: String,
    pub customer_name: Option<String>,
    pub amount_minor: i64,
    pub currency: String,
    pub invoice_id: String,
    pub invoice_url: Option<String>,
}

pub struct BillingPaymentFailureNotification {
    pub account_id: Uuid,
    pub recipient_email: String,
    pub customer_name: Option<String>,
    pub amount_minor: i64,
    pub currency: String,
    pub invoice_id: String,
    pub billing_portal_url: Option<String>,
    pub invoice_url: Option<String>,
}

pub async fn deliver_billing_receipt(
    client: &EmailClient,
    notification: BillingReceiptNotification,
) -> anyhow::Result<()> {
    let command = receipt_command(&notification)?;
    client.send(command).await?;
    Ok(())
}

pub async fn deliver_payment_failure(
    client: &EmailClient,
    notification: BillingPaymentFailureNotification,
) -> anyhow::Result<()> {
    let command = failure_command(&notification)?;
    client.send(command).await?;
    Ok(())
}

pub fn receipt_command(notification: &BillingReceiptNotification) -> anyhow::Result<EmailCommand> {
    let now = Utc::now();
    let correlation_id = format!("billing:invoice_paid:{}", notification.invoice_id);
    let command = EmailCommand {
        context: EmailRequestContext {
            request_id: correlation_id.clone(),
            correlation_id,
            actor_principal_id: notification.account_id.to_string(),
        },
        producer: "billing-service".into(),
        idempotency_key: EmailIdempotencyKey::new(format!(
            "billing:receipt:{}",
            notification.invoice_id
        ))?,
        recipient: EmailRecipient {
            email: notification.recipient_email.clone(),
            name: notification.customer_name.clone(),
        },
        category: EmailCategory::Billing,
        template: EmailTemplate::BillingReceiptV1 {
            customer_name: notification.customer_name.clone(),
            amount_minor: notification.amount_minor,
            currency: notification.currency.clone(),
            invoice_url: notification.invoice_url.clone(),
            provider_name: Some("Stripe".into()),
        },
        deliver_before: now + Duration::hours(24),
    };
    command.validate(now)?;
    Ok(command)
}

pub fn failure_command(
    notification: &BillingPaymentFailureNotification,
) -> anyhow::Result<EmailCommand> {
    let now = Utc::now();
    let correlation_id = format!("billing:payment_failed:{}", notification.invoice_id);
    let command = EmailCommand {
        context: EmailRequestContext {
            request_id: correlation_id.clone(),
            correlation_id,
            actor_principal_id: notification.account_id.to_string(),
        },
        producer: "billing-service".into(),
        idempotency_key: EmailIdempotencyKey::new(format!(
            "billing:payment_failed:{}",
            notification.invoice_id
        ))?,
        recipient: EmailRecipient {
            email: notification.recipient_email.clone(),
            name: notification.customer_name.clone(),
        },
        category: EmailCategory::Billing,
        template: EmailTemplate::BillingPaymentFailureV1 {
            customer_name: notification.customer_name.clone(),
            amount_minor: notification.amount_minor,
            currency: notification.currency.clone(),
            billing_portal_url: notification.billing_portal_url.clone(),
            invoice_url: notification.invoice_url.clone(),
            provider_name: Some("Stripe".into()),
        },
        deliver_before: now + Duration::hours(24),
    };
    command.validate(now)?;
    Ok(command)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_valid_receipt_command() {
        let notif = BillingReceiptNotification {
            account_id: Uuid::new_v4(),
            recipient_email: "test@example.com".into(),
            customer_name: Some("Alice".into()),
            amount_minor: 1000,
            currency: "EUR".into(),
            invoice_id: "in_test_123".into(),
            invoice_url: Some("https://stripe.com/invoice/123".into()),
        };
        let cmd = receipt_command(&notif).expect("command should be valid");
        assert_eq!(cmd.producer, "billing-service");
        assert_eq!(cmd.category, EmailCategory::Billing);
    }

    #[test]
    fn builds_valid_payment_failure_command() {
        let notif = BillingPaymentFailureNotification {
            account_id: Uuid::new_v4(),
            recipient_email: "test@example.com".into(),
            customer_name: Some("Bob".into()),
            amount_minor: 2500,
            currency: "EUR".into(),
            invoice_id: "in_test_456".into(),
            billing_portal_url: Some("https://nvbes.test/billing/portal".into()),
            invoice_url: None,
        };
        let cmd = failure_command(&notif).expect("command should be valid");
        assert_eq!(cmd.producer, "billing-service");
        assert_eq!(cmd.category, EmailCategory::Billing);
    }
}
