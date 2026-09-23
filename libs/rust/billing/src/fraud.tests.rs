use super::{
    CheckoutFraudDecision, CheckoutFraudEnforcementAction, CheckoutFraudInput, CheckoutFraudPolicy,
    assess_checkout_fraud, checkout_fraud_enforcement_action,
};

#[test]
fn mismatched_tor_checkout_blocks() {
    let assessment = assess_checkout_fraud(CheckoutFraudInput {
        policy: CheckoutFraudPolicy::default(),
        network_kind: "tor",
        network_risk_score: 95,
        network_labels: &["tor".to_string()],
        geo_country: Some("FR"),
        billing_country: Some("DE"),
        vat_number: Some("DE123"),
        amount_minor: 25_000,
        existing_provider_customer: false,
        active_paid_customer: false,
        recent_ip_checkouts: 0,
        recent_ip_workspaces: 0,
        recent_workspace_countries: 0,
        recent_payment_methods: 0,
        recent_payment_failures: 0,
        trusted_checkout_assessments: 0,
    });

    assert_eq!(assessment.score, 100);
    assert_eq!(assessment.decision, CheckoutFraudDecision::Block);
    assert!(
        assessment
            .labels
            .contains(&"mismatch:billing_country".to_string())
    );
    assert!(assessment.reasons.contains(&"tor_network".to_string()));
}

#[test]
fn identity_checkout_tor_country_mismatch_blocks_when_enforcement_enabled() {
    let assessment = assess_checkout_fraud(CheckoutFraudInput {
        policy: CheckoutFraudPolicy::default(),
        network_kind: "tor",
        network_risk_score: 95,
        network_labels: &["network:tor".to_string()],
        geo_country: Some("FR"),
        billing_country: Some("DE"),
        vat_number: Some("DE123"),
        amount_minor: 25_000,
        existing_provider_customer: false,
        active_paid_customer: false,
        recent_ip_checkouts: 0,
        recent_ip_workspaces: 0,
        recent_workspace_countries: 0,
        recent_payment_methods: 0,
        recent_payment_failures: 0,
        trusted_checkout_assessments: 0,
    });
    let action = checkout_fraud_enforcement_action(true, assessment.decision);

    assert_eq!(assessment.decision, CheckoutFraudDecision::Block);
    assert_eq!(action, CheckoutFraudEnforcementAction::Block);
    assert!(assessment.reasons.contains(&"tor_network".to_string()));
    assert!(
        assessment
            .reasons
            .contains(&"billing_country_mismatch".to_string())
    );
}

#[test]
fn active_paid_customer_reduces_datacenter_friction() {
    let assessment = assess_checkout_fraud(CheckoutFraudInput {
        policy: CheckoutFraudPolicy::default(),
        network_kind: "datacenter",
        network_risk_score: 70,
        network_labels: &["provider:cloudflare".to_string()],
        geo_country: Some("FR"),
        billing_country: Some("FR"),
        vat_number: Some("FR123"),
        amount_minor: 5_000,
        existing_provider_customer: true,
        active_paid_customer: true,
        recent_ip_checkouts: 0,
        recent_ip_workspaces: 0,
        recent_workspace_countries: 0,
        recent_payment_methods: 0,
        recent_payment_failures: 0,
        trusted_checkout_assessments: 0,
    });

    assert_eq!(assessment.decision, CheckoutFraudDecision::Allow);
    assert!(
        assessment
            .labels
            .contains(&"provider:cloudflare".to_string())
    );
}

#[test]
fn velocity_signals_raise_manual_review() {
    let assessment = assess_checkout_fraud(CheckoutFraudInput {
        policy: CheckoutFraudPolicy::default(),
        network_kind: "unknown",
        network_risk_score: 35,
        network_labels: &[],
        geo_country: Some("FR"),
        billing_country: Some("FR"),
        vat_number: None,
        amount_minor: 5_000,
        existing_provider_customer: false,
        active_paid_customer: false,
        recent_ip_checkouts: 6,
        recent_ip_workspaces: 3,
        recent_workspace_countries: 1,
        recent_payment_methods: 0,
        recent_payment_failures: 0,
        trusted_checkout_assessments: 0,
    });

    assert_eq!(assessment.decision, CheckoutFraudDecision::ManualReview);
    assert!(
        assessment
            .labels
            .contains(&"velocity:ip_workspace_reuse".to_string())
    );
}

#[test]
fn multiple_recent_payment_methods_raise_step_up() {
    let assessment = assess_checkout_fraud(CheckoutFraudInput {
        policy: CheckoutFraudPolicy::default(),
        network_kind: "unknown",
        network_risk_score: 45,
        network_labels: &[],
        geo_country: Some("FR"),
        billing_country: Some("FR"),
        vat_number: None,
        amount_minor: 5_000,
        existing_provider_customer: false,
        active_paid_customer: false,
        recent_ip_checkouts: 0,
        recent_ip_workspaces: 0,
        recent_workspace_countries: 0,
        recent_payment_methods: 3,
        recent_payment_failures: 0,
        trusted_checkout_assessments: 0,
    });

    assert_eq!(assessment.decision, CheckoutFraudDecision::StepUp);
    assert!(
        assessment
            .labels
            .contains(&"payment:multiple_recent_methods".to_string())
    );
    assert!(
        assessment
            .reasons
            .contains(&"multiple_recent_payment_methods".to_string())
    );
}

#[test]
fn trusted_fraud_review_reduces_checkout_friction() {
    let assessment = assess_checkout_fraud(CheckoutFraudInput {
        policy: CheckoutFraudPolicy::default(),
        network_kind: "proxy",
        network_risk_score: 67,
        network_labels: &[],
        geo_country: Some("FR"),
        billing_country: Some("FR"),
        vat_number: None,
        amount_minor: 5_000,
        existing_provider_customer: false,
        active_paid_customer: false,
        recent_ip_checkouts: 0,
        recent_ip_workspaces: 0,
        recent_workspace_countries: 0,
        recent_payment_methods: 0,
        recent_payment_failures: 0,
        trusted_checkout_assessments: 1,
    });

    assert_eq!(assessment.score, 55);
    assert_eq!(assessment.decision, CheckoutFraudDecision::Monitor);
    assert!(
        assessment
            .labels
            .contains(&"customer:trusted_fraud_review".to_string())
    );
}

#[test]
fn disabled_enforcement_only_observes() {
    let action = checkout_fraud_enforcement_action(false, CheckoutFraudDecision::Block);

    assert_eq!(action, CheckoutFraudEnforcementAction::Observe);
}

#[test]
fn enabled_enforcement_keeps_monitor_as_log_only_action() {
    let action = checkout_fraud_enforcement_action(true, CheckoutFraudDecision::Monitor);

    assert_eq!(action, CheckoutFraudEnforcementAction::Monitor);
    assert_eq!(action.as_str(), "monitor");
}

#[test]
fn proxy_and_vpn_networks_add_expected_reasons() {
    let proxy = assess_checkout_fraud(CheckoutFraudInput {
        policy: CheckoutFraudPolicy::default(),
        network_kind: "proxy",
        network_risk_score: 70,
        network_labels: &[],
        geo_country: Some("FR"),
        billing_country: Some("FR"),
        vat_number: None,
        amount_minor: 2_000,
        existing_provider_customer: false,
        active_paid_customer: false,
        recent_ip_checkouts: 0,
        recent_ip_workspaces: 0,
        recent_workspace_countries: 0,
        recent_payment_methods: 0,
        recent_payment_failures: 0,
        trusted_checkout_assessments: 0,
    });
    assert!(proxy.reasons.contains(&"proxy_network".to_string()));

    let vpn = assess_checkout_fraud(CheckoutFraudInput {
        policy: CheckoutFraudPolicy::default(),
        network_kind: "vpn",
        network_risk_score: 65,
        network_labels: &[],
        geo_country: Some("FR"),
        billing_country: Some("FR"),
        vat_number: None,
        amount_minor: 2_000,
        existing_provider_customer: false,
        active_paid_customer: false,
        recent_ip_checkouts: 0,
        recent_ip_workspaces: 0,
        recent_workspace_countries: 0,
        recent_payment_methods: 0,
        recent_payment_failures: 0,
        trusted_checkout_assessments: 0,
    });
    assert!(vpn.reasons.contains(&"vpn_network".to_string()));
}

#[test]
fn high_checkout_amount_and_payment_failures_raise_risk() {
    let assessment = assess_checkout_fraud(CheckoutFraudInput {
        policy: CheckoutFraudPolicy::default(),
        network_kind: "unknown",
        network_risk_score: 10,
        network_labels: &[],
        geo_country: Some("FR"),
        billing_country: Some("FR"),
        vat_number: None,
        amount_minor: 75_000,
        existing_provider_customer: false,
        active_paid_customer: false,
        recent_ip_checkouts: 0,
        recent_ip_workspaces: 0,
        recent_workspace_countries: 0,
        recent_payment_methods: 0,
        recent_payment_failures: 4,
        trusted_checkout_assessments: 0,
    });
    assert!(assessment.score >= 40);
    assert!(
        assessment
            .labels
            .iter()
            .any(|label| label.contains("payment") || label.contains("amount"))
    );
}

#[test]
fn enabled_enforcement_holds_manual_review_and_blocks_block() {
    assert_eq!(
        checkout_fraud_enforcement_action(true, CheckoutFraudDecision::ManualReview),
        CheckoutFraudEnforcementAction::ManualReviewHold
    );
    assert_eq!(
        checkout_fraud_enforcement_action(true, CheckoutFraudDecision::Block),
        CheckoutFraudEnforcementAction::Block
    );
}
