use super::*;

#[test]
fn receipt_command_carries_optional_invoice_url() {
    let notif = BillingReceiptNotification {
        account_id: Uuid::new_v4(),
        recipient_email: "test@example.com".into(),
        customer_name: None,
        amount_minor: 500,
        currency: "EUR".into(),
        invoice_id: "in_url".into(),
        invoice_url: Some("https://stripe.test/in_url".into()),
    };
    let cmd = receipt_command(&notif).expect("valid");
    match cmd.template {
        EmailTemplate::BillingReceiptV1 { invoice_url, .. } => {
            assert_eq!(invoice_url.as_deref(), Some("https://stripe.test/in_url"));
        }
        _ => panic!("expected billing receipt template"),
    }
}

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
    assert_eq!(cmd.producer, "billing-worker");
    assert_eq!(cmd.category, EmailCategory::Billing);
    match cmd.template {
        EmailTemplate::BillingReceiptV1 {
            amount_minor,
            currency,
            ..
        } => {
            assert_eq!(amount_minor, 1000);
            assert_eq!(currency, "EUR");
        }
        _ => panic!("expected billing receipt template"),
    }
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
    assert_eq!(cmd.producer, "billing-worker");
    assert_eq!(cmd.category, EmailCategory::Billing);
    assert!(
        cmd.context
            .correlation_id
            .contains("billing:payment_failed:in_test_456")
    );
}

#[test]
fn rejects_invalid_recipient_email_for_receipt_and_failure() {
    let receipt = BillingReceiptNotification {
        account_id: Uuid::new_v4(),
        recipient_email: "not-an-email".into(),
        customer_name: None,
        amount_minor: 1,
        currency: "EUR".into(),
        invoice_id: "in_bad".into(),
        invoice_url: None,
    };
    assert!(receipt_command(&receipt).is_err());

    let failure = BillingPaymentFailureNotification {
        account_id: Uuid::new_v4(),
        recipient_email: "".into(),
        customer_name: None,
        amount_minor: 1,
        currency: "EUR".into(),
        invoice_id: "in_bad".into(),
        billing_portal_url: None,
        invoice_url: None,
    };
    assert!(failure_command(&failure).is_err());
}

#[test]
fn receipt_command_rejects_overlong_invoice_identifier() {
    let receipt = BillingReceiptNotification {
        account_id: Uuid::new_v4(),
        recipient_email: "user@example.com".into(),
        customer_name: None,
        amount_minor: 100,
        currency: "EUR".into(),
        invoice_id: "x".repeat(300),
        invoice_url: None,
    };
    assert!(receipt_command(&receipt).is_err());
}
