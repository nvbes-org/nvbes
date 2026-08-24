use std::collections::{BTreeMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::types::{Recommendation, RiskBand};

pub type FeatureMap = BTreeMap<String, f64>;

const MAX_RULES: usize = 256;
const MAX_PREDICATE_DEPTH: usize = 8;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleSet {
    version: String,
    feature_version: String,
    thresholds: Thresholds,
    rules: Vec<Rule>,
}

impl RuleSet {
    pub fn from_json(bytes: &[u8]) -> Result<Self, RuleSetError> {
        let rules: Self = serde_json::from_slice(bytes).map_err(RuleSetError::Json)?;
        rules.validate()?;
        Ok(rules)
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn feature_version(&self) -> &str {
        &self.feature_version
    }

    fn validate(&self) -> Result<(), RuleSetError> {
        validate_key(&self.version)?;
        validate_key(&self.feature_version)?;
        self.thresholds.validate()?;
        if self.rules.is_empty() || self.rules.len() > MAX_RULES {
            return Err(RuleSetError::RuleCount);
        }
        let mut rule_codes = HashSet::new();
        let mut reason_codes = HashSet::new();
        for rule in &self.rules {
            validate_key(&rule.code)?;
            if !rule_codes.insert(&rule.code) {
                return Err(RuleSetError::DuplicateRule);
            }
            if !(-100..=100).contains(&rule.score_delta) {
                return Err(RuleSetError::ScoreDelta);
            }
            if rule.reasons.is_empty() || rule.reasons.len() > 8 {
                return Err(RuleSetError::ReasonCount);
            }
            for reason in &rule.reasons {
                validate_key(reason)?;
                if !reason_codes.insert(reason) {
                    return Err(RuleSetError::DuplicateReason);
                }
            }
            rule.predicate.validate(1)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Thresholds {
    challenge: u8,
    review: u8,
    deny: u8,
}

impl Thresholds {
    fn validate(self) -> Result<(), RuleSetError> {
        if self.challenge <= self.review && self.review <= self.deny && self.deny <= 100 {
            Ok(())
        } else {
            Err(RuleSetError::Thresholds)
        }
    }

    fn recommendation(self, score: u8) -> Recommendation {
        if score >= self.deny {
            Recommendation::Deny
        } else if score >= self.review {
            Recommendation::Review
        } else if score >= self.challenge {
            Recommendation::Challenge
        } else {
            Recommendation::Allow
        }
    }

    fn band(self, score: u8) -> RiskBand {
        if score >= self.deny {
            RiskBand::Critical
        } else if score >= self.review {
            RiskBand::High
        } else if score >= self.challenge {
            RiskBand::Elevated
        } else {
            RiskBand::Low
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Rule {
    code: String,
    predicate: Predicate,
    score_delta: i16,
    reasons: Vec<String>,
    minimum_recommendation: Option<Recommendation>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
enum Predicate {
    All { predicates: Vec<Predicate> },
    Any { predicates: Vec<Predicate> },
    Not { predicate: Box<Predicate> },
    Gte { feature: String, value: f64 },
    Gt { feature: String, value: f64 },
    Lte { feature: String, value: f64 },
    Lt { feature: String, value: f64 },
    Eq { feature: String, value: f64 },
}

impl Predicate {
    fn validate(&self, depth: usize) -> Result<(), RuleSetError> {
        if depth > MAX_PREDICATE_DEPTH {
            return Err(RuleSetError::PredicateDepth);
        }
        match self {
            Self::All { predicates } | Self::Any { predicates } => {
                if predicates.is_empty() || predicates.len() > 16 {
                    return Err(RuleSetError::PredicateCount);
                }
                for predicate in predicates {
                    predicate.validate(depth + 1)?;
                }
            }
            Self::Not { predicate } => predicate.validate(depth + 1)?,
            Self::Gte { feature, value }
            | Self::Gt { feature, value }
            | Self::Lte { feature, value }
            | Self::Lt { feature, value }
            | Self::Eq { feature, value } => {
                validate_key(feature)?;
                if !value.is_finite() {
                    return Err(RuleSetError::ThresholdValue);
                }
            }
        }
        Ok(())
    }

    fn matches(&self, features: &FeatureMap) -> bool {
        match self {
            Self::All { predicates } => predicates.iter().all(|item| item.matches(features)),
            Self::Any { predicates } => predicates.iter().any(|item| item.matches(features)),
            Self::Not { predicate } => !predicate.matches(features),
            Self::Gte { feature, value } => feature_value(features, feature) >= *value,
            Self::Gt { feature, value } => feature_value(features, feature) > *value,
            Self::Lte { feature, value } => feature_value(features, feature) <= *value,
            Self::Lt { feature, value } => feature_value(features, feature) < *value,
            Self::Eq { feature, value } => {
                (feature_value(features, feature) - *value).abs() <= f64::EPSILON
            }
        }
    }
}

fn feature_value(features: &FeatureMap, feature: &str) -> f64 {
    features.get(feature).copied().unwrap_or_default()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluationResult {
    pub score: u8,
    pub band: RiskBand,
    pub recommendation: Recommendation,
    pub reason_codes: Vec<String>,
}

pub fn evaluate(rule_set: &RuleSet, features: &FeatureMap) -> EvaluationResult {
    let mut score = 0_i16;
    let mut minimum = Recommendation::Allow;
    let mut reason_codes = Vec::new();
    let mut seen_reasons = HashSet::new();
    for rule in &rule_set.rules {
        if !rule.predicate.matches(features) {
            continue;
        }
        score += rule.score_delta;
        if let Some(recommendation) = rule.minimum_recommendation {
            minimum = minimum.strictest(recommendation);
        }
        for reason in &rule.reasons {
            if seen_reasons.insert(reason.as_str()) {
                reason_codes.push(reason.clone());
            }
        }
    }
    let score = score.clamp(0, 100) as u8;
    let recommendation = rule_set.thresholds.recommendation(score).strictest(minimum);
    EvaluationResult {
        score,
        band: rule_set.thresholds.band(score),
        recommendation,
        reason_codes,
    }
}

fn validate_key(value: &str) -> Result<(), RuleSetError> {
    if value.is_empty()
        || value.len() > 80
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'_' | b'-' | b'.' | b':')
        })
    {
        return Err(RuleSetError::InvalidKey);
    }
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum RuleSetError {
    #[error("rule set JSON is invalid")]
    Json(#[source] serde_json::Error),
    #[error("rule set key is invalid")]
    InvalidKey,
    #[error("rule set thresholds are invalid")]
    Thresholds,
    #[error("rule count is invalid")]
    RuleCount,
    #[error("rule code is duplicated")]
    DuplicateRule,
    #[error("rule score delta is invalid")]
    ScoreDelta,
    #[error("rule reason count is invalid")]
    ReasonCount,
    #[error("rule reason is duplicated")]
    DuplicateReason,
    #[error("predicate nesting is too deep")]
    PredicateDepth,
    #[error("predicate count is invalid")]
    PredicateCount,
    #[error("predicate threshold is invalid")]
    ThresholdValue,
}
