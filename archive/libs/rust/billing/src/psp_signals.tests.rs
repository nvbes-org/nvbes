use super::{mollie_payment_signal, stripe_invoice_payment_failed_signal};
use crate::provider::{ProviderCode, ProviderPayment, ProviderPaymentMethod};

#[test]
fn stripe_failed_invoice_extracts_safe_radar_and_3ds_hints() {
    let signal = stripe_invoice_payment_failed_signal(&serde_json::json!({
        "id": "in_123",
        "payment_intent": "pi_123",
        "amount_due": 2500,
        "currency": "eur",
        "metadata": { "plan_code": "team" },
        "charge": {
            "payment_method": "pm_123",
            "outcome": {
                "risk_level": "elevated",
                "risk_score": 72,
                "network_status": "declined_by_network"
            },
            "payment_method_details": {
                "card": {
                    "three_d_secure": { "result": "failed" }
                }
            }
        }
    }));

    assert_eq!(signal.provider, "stripe");
    assert_eq!(signal.provider_payment_id.as_deref(), Some("pi_123"));
    assert_eq!(signal.provider_payment_status, "failed");
    assert_eq!(signal.plan_code.as_deref(), Some("team"));
    assert_eq!(signal.currency.as_deref(), Some("EUR"));
    assert!(
        signal
            .labels
            .contains(&"stripe_radar:risk_level_elevated".to_string())
    );
    assert!(signal.labels.contains(&"three_d_secure:failed".to_string()));
    assert_eq!(signal.metadata["stripe_radar_risk_score"], 72);
    assert_eq!(signal.metadata["provider_payment_method_id"], "pm_123");
}

#[test]
fn mollie_failed_payment_extracts_safe_method_hints() {
    let signal = mollie_payment_signal(&ProviderPayment {
        provider: ProviderCode::Mollie,
        provider_payment_id: "tr_123".to_string(),
        provider_customer_id: Some("cst_123".to_string()),
        provider_subscription_id: None,
        plan_code: Some("team".to_string()),
        status: "failed".to_string(),
        amount_minor: 2500,
        currency: "eur".to_string(),
        payment_method: Some(ProviderPaymentMethod {
            method_type: "card".to_string(),
            brand: Some("visa".to_string()),
            last4: Some("1234".to_string()),
            exp_month: Some(12),
            exp_year: Some(2028),
            funding: Some("credit".to_string()),
            issuer_country: Some("FR".to_string()),
            fingerprint: Some("fp_123".to_string()),
            provider_payment_method_id: Some("mdt_123".to_string()),
            mandate_id: Some("mdt_123".to_string()),
            mandate_status: "unknown".to_string(),
            reusable: false,
        }),
    })
    .expect("failed payment should produce a fraud signal");

    assert_eq!(signal.provider, "mollie");
    assert_eq!(signal.provider_payment_status, "failed");
    assert_eq!(signal.metadata["provider_payment_method"], "card");
    assert_eq!(signal.metadata["provider_payment_method_id"], "mdt_123");
    assert_eq!(signal.metadata["card_issuer_country"], "FR");
}

#[test]
fn mollie_chargeback_is_recorded_as_disputed_fraud_signal() {
    let signal = mollie_payment_signal(&ProviderPayment {
        provider: ProviderCode::Mollie,
        provider_payment_id: "tr_chargeback".to_string(),
        provider_customer_id: Some("cst_123".to_string()),
        provider_subscription_id: None,
        plan_code: Some("team".to_string()),
        status: "charged_back".to_string(),
        amount_minor: 2500,
        currency: "eur".to_string(),
        payment_method: None,
    })
    .expect("chargeback should produce a fraud signal");

    assert_eq!(signal.provider, "mollie");
    assert_eq!(signal.provider_payment_status, "disputed");
    assert!(
        signal
            .labels
            .contains(&"payment:provider_charged_back".to_string())
    );
    assert!(
        signal
            .reasons
            .contains(&"provider_payment_charged_back".to_string())
    );
}

#[test]
fn mollie_successful_payment_does_not_emit_fraud_signal() {
    assert!(
        mollie_payment_signal(&ProviderPayment {
            provider: ProviderCode::Mollie,
            provider_payment_id: "tr_ok".to_string(),
            provider_customer_id: None,
            provider_subscription_id: None,
            plan_code: None,
            status: "captured".to_string(),
            amount_minor: 100,
            currency: "EUR".to_string(),
            payment_method: None,
        })
        .is_none()
    );
}

#[test]
fn psp_score_bands_cover_disputed_failed_and_canceled_paths() {
    let disputed = mollie_payment_signal(&ProviderPayment {
        provider: ProviderCode::Mollie,
        provider_payment_id: "tr_disputed".to_string(),
        provider_customer_id: None,
        provider_subscription_id: None,
        plan_code: None,
        status: "disputed".to_string(),
        amount_minor: 100,
        currency: "EUR".to_string(),
        payment_method: None,
    })
    .expect("disputed");
    assert_eq!(disputed.provider_payment_status, "disputed");
    assert_eq!(disputed.score, 75);

    let canceled = mollie_payment_signal(&ProviderPayment {
        provider: ProviderCode::Mollie,
        provider_payment_id: "tr_canceled".to_string(),
        provider_customer_id: None,
        provider_subscription_id: None,
        plan_code: None,
        status: "canceled".to_string(),
        amount_minor: 100,
        currency: "EUR".to_string(),
        payment_method: None,
    })
    .expect("canceled");
    assert_eq!(canceled.provider_payment_status, "canceled");
    assert_eq!(canceled.score, 30);

    let highest = stripe_invoice_payment_failed_signal(&serde_json::json!({
        "id": "in_high",
        "amount_due": 1000,
        "currency": "eur",
        "charge": { "outcome": { "risk_level": "highest" } }
    }));
    assert_eq!(highest.score, 70);

    let elevated = stripe_invoice_payment_failed_signal(&serde_json::json!({
        "id": "in_elevated",
        "amount_due": 1000,
        "currency": "eur",
        "charge": { "outcome": { "risk_level": "elevated" } }
    }));
    assert_eq!(elevated.score, 60);

    let plain_failed = stripe_invoice_payment_failed_signal(&serde_json::json!({
        "id": "in_plain_failed",
        "amount_due": 1000,
        "currency": "eur"
    }));
    assert_eq!(plain_failed.score, 45);
}

#[test]
fn stripe_failed_invoice_without_radar_or_3ds_stays_minimal() {
    let signal = stripe_invoice_payment_failed_signal(&serde_json::json!({
        "id": "in_plain",
        "amount_remaining": 1000,
        "currency": "usd"
    }));
    assert_eq!(signal.provider_payment_id.as_deref(), Some("in_plain"));
    assert_eq!(signal.amount_minor, Some(1000));
    assert_eq!(signal.currency.as_deref(), Some("USD"));
    assert!(
        !signal
            .labels
            .iter()
            .any(|label| label.contains("stripe_radar"))
    );
    assert!(
        !signal
            .labels
            .iter()
            .any(|label| label.contains("three_d_secure"))
    );
}
