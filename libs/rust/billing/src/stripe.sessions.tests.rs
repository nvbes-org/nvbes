use uuid::Uuid;

use super::{
    StripeCheckoutSessionParams, build_checkout_session_fields, create_stripe_checkout_session,
    create_stripe_portal_session, stripe_session_from_response,
};
use crate::stripe::StripeProviderError;
use nvbes_core::config::AppConfig;

fn checkout_params<'a>(fraud_metadata: &'a [(String, String)]) -> StripeCheckoutSessionParams<'a> {
    StripeCheckoutSessionParams {
        customer_id: "cus_test",
        owner_principal_id: Uuid::nil(),
        workspace_id: Uuid::from_u128(42),
        plan_code: "team",
        stripe_price_id: "price_test",
        success_url: "https://app.example/success",
        cancel_url: "https://app.example/cancel",
        fraud_metadata,
    }
}

#[test]
fn build_checkout_session_fields_include_subscription_metadata() {
    let fields = build_checkout_session_fields(checkout_params(&[]));
    let map = fields
        .into_iter()
        .collect::<std::collections::HashMap<_, _>>();

    assert_eq!(map.get("mode"), Some(&"subscription".to_string()));
    assert_eq!(map.get("customer"), Some(&"cus_test".to_string()));
    assert_eq!(
        map.get("metadata[workspace_id]"),
        Some(&Uuid::from_u128(42).to_string())
    );
    assert_eq!(map.get("allow_promotion_codes"), Some(&"true".to_string()));
    assert_eq!(map.get("automatic_tax[enabled]"), Some(&"true".to_string()));
}

#[test]
fn build_checkout_session_fields_adds_step_up_when_enforcement_requires_it() {
    let fraud = [("enforcement_action".to_string(), "step_up".to_string())];
    let fields = build_checkout_session_fields(checkout_params(&fraud));
    assert!(
        fields.iter().any(|(key, value)| {
            key == "payment_method_options[card][request_three_d_secure]" && value == "challenge"
        }),
        "expected 3DS challenge field"
    );
    assert!(
        fields
            .iter()
            .any(|(key, _)| key == "metadata[enforcement_action]"),
        "fraud metadata copied to session metadata"
    );
}

#[test]
fn build_checkout_session_fields_skips_3ds_without_step_up_enforcement() {
    let fraud = [("enforcement_action".to_string(), "monitor".to_string())];
    let fields = build_checkout_session_fields(checkout_params(&fraud));
    assert!(
        !fields
            .iter()
            .any(|(key, _)| key == "payment_method_options[card][request_three_d_secure]"),
        "monitor enforcement should not request 3DS challenge"
    );
}

#[test]
fn stripe_session_from_response_requires_id_and_url() {
    let err = stripe_session_from_response(serde_json::json!({ "url": "https://stripe.test" }))
        .unwrap_err();
    assert!(matches!(err, StripeProviderError::ResponseInvalid(_)));

    let err = stripe_session_from_response(serde_json::json!({ "id": "cs_test" })).unwrap_err();
    assert!(matches!(err, StripeProviderError::ResponseInvalid(_)));

    let session = stripe_session_from_response(serde_json::json!({
        "id": "cs_test",
        "url": "https://checkout.stripe.test"
    }))
    .expect("valid session");
    assert_eq!(session.id, "cs_test");
    assert_eq!(session.url, "https://checkout.stripe.test");
}

#[tokio::test]
async fn create_stripe_checkout_session_requires_secret_key() {
    let config = AppConfig {
        stripe_secret_key: None,
        ..AppConfig::default()
    };
    let err = create_stripe_checkout_session(&config, checkout_params(&[]))
        .await
        .unwrap_err();
    assert!(matches!(err, StripeProviderError::NotConfigured));
}

#[tokio::test]
async fn create_stripe_portal_session_rejects_live_keys() {
    let config = AppConfig {
        stripe_secret_key: Some("sk_live_forbidden".to_string()),
        ..AppConfig::default()
    };
    let err = create_stripe_portal_session(&config, "cus_test", "https://app.example/billing")
        .await
        .unwrap_err();
    assert!(matches!(err, StripeProviderError::LiveKeyRejected));
}
