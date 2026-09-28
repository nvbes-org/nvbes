use uuid::Uuid;

use super::{
    BillingPaymentFailureNotification, BillingReceiptNotification, failure_command, receipt_command,
};
use nvbes_email::EmailCategory;

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
