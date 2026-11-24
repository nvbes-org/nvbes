use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::fraud::CheckoutFraudPolicy;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckoutFraudPolicyOverride {
    pub provider: Option<String>,
    pub plan_code: Option<String>,
    pub country: Option<String>,
    pub min_amount_minor: Option<i64>,
    pub max_amount_minor: Option<i64>,
    pub step_up_threshold: Option<u8>,
    pub manual_review_threshold: Option<u8>,
    pub block_threshold: Option<u8>,
}

#[derive(Debug, Clone, Copy)]
pub struct CheckoutFraudPolicyContext<'a> {
    pub provider: Option<&'a str>,
    pub plan_code: &'a str,
    pub country: Option<&'a str>,
    pub amount_minor: i64,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CheckoutFraudPolicyError {
    #[error("billing fraud policy overrides JSON is invalid")]
    InvalidJson,
    #[error("billing fraud policy override amount range is invalid")]
    InvalidAmountRange,
    #[error(
        "billing fraud policy thresholds must satisfy step_up <= manual_review <= block <= 100"
    )]
    InvalidThresholds,
}

pub fn resolve_checkout_fraud_policy(
    base: CheckoutFraudPolicy,
    overrides_json: Option<&str>,
    context: CheckoutFraudPolicyContext<'_>,
) -> Result<CheckoutFraudPolicy, CheckoutFraudPolicyError> {
    let Some(overrides_json) = overrides_json.filter(|value| !value.trim().is_empty()) else {
        return Ok(base);
    };
    let overrides: Vec<CheckoutFraudPolicyOverride> =
        serde_json::from_str(overrides_json).map_err(|_| CheckoutFraudPolicyError::InvalidJson)?;
    let mut selected: Option<(usize, usize, &CheckoutFraudPolicyOverride)> = None;
    for (index, override_policy) in overrides.iter().enumerate() {
        validate_override(override_policy)?;
        if !override_matches(override_policy, context) {
            continue;
        }
        let specificity = override_specificity(override_policy);
        if selected
            .map(|(current_specificity, current_index, _)| {
                (specificity, index) > (current_specificity, current_index)
            })
            .unwrap_or(true)
        {
            selected = Some((specificity, index, override_policy));
        }
    }
    let Some((_, _, selected)) = selected else {
        return Ok(base);
    };
    let policy = CheckoutFraudPolicy {
        step_up_threshold: selected.step_up_threshold.unwrap_or(base.step_up_threshold),
        manual_review_threshold: selected
            .manual_review_threshold
            .unwrap_or(base.manual_review_threshold),
        block_threshold: selected.block_threshold.unwrap_or(base.block_threshold),
    };
    if !policy.is_valid() {
        return Err(CheckoutFraudPolicyError::InvalidThresholds);
    }
    Ok(policy)
}

pub fn validate_checkout_fraud_policy_overrides(
    base: CheckoutFraudPolicy,
    overrides_json: Option<&str>,
) -> Result<(), CheckoutFraudPolicyError> {
    let Some(overrides_json) = overrides_json.filter(|value| !value.trim().is_empty()) else {
        return Ok(());
    };
    let overrides: Vec<CheckoutFraudPolicyOverride> =
        serde_json::from_str(overrides_json).map_err(|_| CheckoutFraudPolicyError::InvalidJson)?;
    for override_policy in overrides {
        validate_override(&override_policy)?;
        let policy = CheckoutFraudPolicy {
            step_up_threshold: override_policy
                .step_up_threshold
                .unwrap_or(base.step_up_threshold),
            manual_review_threshold: override_policy
                .manual_review_threshold
                .unwrap_or(base.manual_review_threshold),
            block_threshold: override_policy
                .block_threshold
                .unwrap_or(base.block_threshold),
        };
        if !policy.is_valid() {
            return Err(CheckoutFraudPolicyError::InvalidThresholds);
        }
    }
    Ok(())
}

fn validate_override(
    override_policy: &CheckoutFraudPolicyOverride,
) -> Result<(), CheckoutFraudPolicyError> {
    if matches!(
        (override_policy.min_amount_minor, override_policy.max_amount_minor),
        (Some(min), Some(max)) if min > max
    ) {
        return Err(CheckoutFraudPolicyError::InvalidAmountRange);
    }
    if [
        override_policy.step_up_threshold,
        override_policy.manual_review_threshold,
        override_policy.block_threshold,
    ]
    .into_iter()
    .flatten()
    .any(|threshold| threshold > 100)
    {
        return Err(CheckoutFraudPolicyError::InvalidThresholds);
    }
    Ok(())
}

fn override_matches(
    override_policy: &CheckoutFraudPolicyOverride,
    context: CheckoutFraudPolicyContext<'_>,
) -> bool {
    string_matches(override_policy.provider.as_deref(), context.provider)
        && string_matches(
            override_policy.plan_code.as_deref(),
            Some(context.plan_code),
        )
        && string_matches(override_policy.country.as_deref(), context.country)
        && override_policy
            .min_amount_minor
            .map(|min| context.amount_minor >= min)
            .unwrap_or(true)
        && override_policy
            .max_amount_minor
            .map(|max| context.amount_minor <= max)
            .unwrap_or(true)
}

fn string_matches(expected: Option<&str>, actual: Option<&str>) -> bool {
    expected
        .map(|expected| {
            actual
                .map(|actual| expected.eq_ignore_ascii_case(actual))
                .unwrap_or(false)
        })
        .unwrap_or(true)
}

fn override_specificity(override_policy: &CheckoutFraudPolicyOverride) -> usize {
    [
        override_policy.provider.is_some(),
        override_policy.plan_code.is_some(),
        override_policy.country.is_some(),
        override_policy.min_amount_minor.is_some(),
        override_policy.max_amount_minor.is_some(),
    ]
    .into_iter()
    .filter(|matched| *matched)
    .count()
}

#[cfg(test)]
#[path = "fraud_policy.tests.rs"]
mod tests;
