use nvbes_core::config::AppConfig;
use uuid::Uuid;

use crate::checkout_sessions::BillingCheckoutSessionError;
use crate::db::{
    BillingFraudAssessmentInput, BillingFraudVelocity, insert_billing_fraud_assessment_tx,
};
use crate::models::{BillingStateRecord, PlanRecord};
use crate::provider::ProviderCode;
use crate::{
    CheckoutFraudAssessment, CheckoutFraudEnforcementAction, CheckoutFraudInput,
    CheckoutFraudPolicy, CheckoutFraudPolicyContext, assess_checkout_fraud,
    checkout_fraud_enforcement_action, resolve_checkout_fraud_policy,
};

pub(crate) struct AssessCheckoutFraudSessionParams<'a> {
    pub config: &'a AppConfig,
    pub record: &'a BillingStateRecord,
    pub target_plan: &'a PlanRecord,
    pub provider: ProviderCode,
    pub checkout_country: Option<&'a str>,
    pub checkout_amount_minor: i64,
    pub geo_resolution: &'a nvbes_region::geo::GeoResolution,
    pub velocity: BillingFraudVelocity,
}

pub(crate) fn assess_checkout_session_fraud(
    params: AssessCheckoutFraudSessionParams<'_>,
) -> Result<CheckoutFraudAssessment, BillingCheckoutSessionError> {
    Ok(assess_checkout_fraud(CheckoutFraudInput {
        policy: checkout_fraud_policy(
            params.config,
            CheckoutFraudPolicyContext {
                provider: Some(params.provider.as_str()),
                plan_code: &params.target_plan.code,
                country: params.checkout_country,
                amount_minor: params.checkout_amount_minor,
            },
        )?,
        network_kind: params.geo_resolution.network_kind.as_str(),
        network_risk_score: params.geo_resolution.risk_score,
        network_labels: &params.geo_resolution.risk_labels,
        geo_country: params.checkout_country,
        billing_country: params.record.country.as_deref(),
        vat_number: params.record.vat_number.as_deref(),
        amount_minor: params.checkout_amount_minor,
        existing_provider_customer: params.record.provider_customer_id.is_some()
            || params.record.stripe_customer_id.is_some()
            || params.record.billing_customer_id.is_some(),
        active_paid_customer: params.record.subscription_status == "active",
        recent_ip_checkouts: params.velocity.recent_ip_checkouts,
        recent_ip_workspaces: params.velocity.recent_ip_workspaces,
        recent_workspace_countries: params.velocity.recent_workspace_countries,
        recent_payment_methods: params.velocity.recent_payment_methods,
        recent_payment_failures: params.velocity.recent_payment_failures,
        trusted_checkout_assessments: params.velocity.trusted_checkout_assessments,
    }))
}

pub(crate) fn checkout_fraud_enforcement(
    config: &AppConfig,
    assessment: &CheckoutFraudAssessment,
) -> CheckoutFraudEnforcementAction {
    checkout_fraud_enforcement_action(
        config.billing_fraud_enforcement_enabled,
        assessment.decision,
    )
}

pub(crate) fn enforce_checkout_fraud(
    action: CheckoutFraudEnforcementAction,
) -> Result<(), BillingCheckoutSessionError> {
    match action {
        CheckoutFraudEnforcementAction::Observe
        | CheckoutFraudEnforcementAction::Allow
        | CheckoutFraudEnforcementAction::Monitor
        | CheckoutFraudEnforcementAction::StepUp => Ok(()),
        CheckoutFraudEnforcementAction::ManualReviewHold => {
            Err(BillingCheckoutSessionError::ManualReviewHold)
        }
        CheckoutFraudEnforcementAction::Block => Err(BillingCheckoutSessionError::FraudBlocked),
    }
}

pub(crate) fn fraud_provider_metadata(
    assessment: &CheckoutFraudAssessment,
    action: CheckoutFraudEnforcementAction,
) -> Vec<(String, String)> {
    vec![
        ("fraud_score".to_string(), assessment.score.to_string()),
        (
            "fraud_decision".to_string(),
            assessment.decision.as_str().to_string(),
        ),
        (
            "network_threat".to_string(),
            assessment.network_threat.as_str().to_string(),
        ),
        (
            "enforcement_action".to_string(),
            action.as_str().to_string(),
        ),
    ]
}

fn checkout_fraud_policy(
    config: &AppConfig,
    context: CheckoutFraudPolicyContext<'_>,
) -> Result<CheckoutFraudPolicy, BillingCheckoutSessionError> {
    resolve_checkout_fraud_policy(
        CheckoutFraudPolicy {
            step_up_threshold: config.billing_fraud_step_up_threshold,
            manual_review_threshold: config.billing_fraud_manual_review_threshold,
            block_threshold: config.billing_fraud_block_threshold,
        },
        config.billing_fraud_policy_overrides_json.as_deref(),
        context,
    )
    .map_err(BillingCheckoutSessionError::FraudPolicy)
}

pub(crate) struct CheckoutFraudAssessmentRecord<'a> {
    pub workspace_id: Uuid,
    pub actor_principal_id: Uuid,
    pub record: &'a BillingStateRecord,
    pub target_plan: &'a PlanRecord,
    pub checkout_amount_minor: i64,
    pub ip_address: Option<&'a str>,
    pub checkout_country: Option<&'a str>,
    pub provider: Option<&'a str>,
    pub checkout_id: Option<&'a str>,
    pub payment_id: Option<&'a str>,
    pub geo_resolution: &'a nvbes_region::geo::GeoResolution,
    pub assessment: &'a CheckoutFraudAssessment,
    pub action: CheckoutFraudEnforcementAction,
    pub velocity: BillingFraudVelocity,
}

pub(crate) async fn record_checkout_fraud_assessment_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    input: CheckoutFraudAssessmentRecord<'_>,
) -> Result<Uuid, sqlx::Error> {
    insert_billing_fraud_assessment_tx(
        tx,
        BillingFraudAssessmentInput {
            workspace_id: input.workspace_id,
            actor_principal_id: Some(input.actor_principal_id),
            provider: input.provider,
            checkout_id: input.checkout_id,
            payment_id: input.payment_id,
            plan_code: &input.target_plan.code,
            amount_minor: input.checkout_amount_minor,
            currency: "EUR",
            ip_address: input.ip_address,
            geo_country_code: input.checkout_country,
            billing_country_code: input.record.country.as_deref(),
            network_kind: input.geo_resolution.network_kind.as_str(),
            network_risk_score: input.geo_resolution.risk_score,
            network_risk_labels: &input.geo_resolution.risk_labels,
            fraud_score: input.assessment.score,
            fraud_decision: input.assessment.decision.as_str(),
            network_threat: input.assessment.network_threat.as_str(),
            fraud_labels: &input.assessment.labels,
            fraud_reasons: &input.assessment.reasons,
            enforcement_action: input.action.as_str(),
            metadata: serde_json::json!({
                "velocity": {
                    "recent_ip_checkouts": input.velocity.recent_ip_checkouts,
                    "recent_ip_workspaces": input.velocity.recent_ip_workspaces,
                    "recent_workspace_countries": input.velocity.recent_workspace_countries,
                    "recent_payment_methods": input.velocity.recent_payment_methods,
                    "recent_payment_failures": input.velocity.recent_payment_failures,
                    "trusted_checkout_assessments": input.velocity.trusted_checkout_assessments
                }
            }),
        },
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::{enforce_checkout_fraud, fraud_provider_metadata};
    use crate::{
        CheckoutFraudAssessment, CheckoutFraudDecision, CheckoutFraudEnforcementAction,
        NetworkThreatLevel,
    };

    #[test]
    fn checkout_fraud_enforcement_blocks_terminal_actions() {
        assert!(enforce_checkout_fraud(CheckoutFraudEnforcementAction::Allow).is_ok());
        assert!(enforce_checkout_fraud(CheckoutFraudEnforcementAction::Monitor).is_ok());
        assert!(enforce_checkout_fraud(CheckoutFraudEnforcementAction::StepUp).is_ok());
        assert!(enforce_checkout_fraud(CheckoutFraudEnforcementAction::ManualReviewHold).is_err());
        assert!(enforce_checkout_fraud(CheckoutFraudEnforcementAction::Block).is_err());
    }

    #[test]
    fn provider_metadata_contains_safe_fraud_fields_only() {
        let metadata = fraud_provider_metadata(
            &CheckoutFraudAssessment {
                score: 88,
                decision: CheckoutFraudDecision::ManualReview,
                network_threat: NetworkThreatLevel::High,
                labels: vec!["tor".to_string()],
                reasons: vec!["tor_network".to_string()],
            },
            CheckoutFraudEnforcementAction::ManualReviewHold,
        );

        assert!(metadata.contains(&("fraud_score".to_string(), "88".to_string())));
        assert!(metadata.contains(&("fraud_decision".to_string(), "manual_review".to_string())));
        assert!(metadata.contains(&("network_threat".to_string(), "high".to_string())));
        assert!(metadata.contains(&(
            "enforcement_action".to_string(),
            "manual_review_hold".to_string()
        )));
    }
}
