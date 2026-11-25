use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkThreatLevel {
    Low,
    Elevated,
    High,
    Critical,
}

impl NetworkThreatLevel {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Elevated => "elevated",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheckoutFraudDecision {
    Allow,
    Monitor,
    StepUp,
    ManualReview,
    Block,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheckoutFraudEnforcementAction {
    Observe,
    Allow,
    Monitor,
    StepUp,
    ManualReviewHold,
    Block,
}

impl CheckoutFraudEnforcementAction {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Observe => "observe",
            Self::Allow => "allow",
            Self::Monitor => "monitor",
            Self::StepUp => "step_up",
            Self::ManualReviewHold => "manual_review_hold",
            Self::Block => "block",
        }
    }
}

impl CheckoutFraudDecision {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Monitor => "monitor",
            Self::StepUp => "step_up",
            Self::ManualReview => "manual_review",
            Self::Block => "block",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CheckoutFraudInput<'a> {
    pub policy: CheckoutFraudPolicy,
    pub network_kind: &'a str,
    pub network_risk_score: u8,
    pub network_labels: &'a [String],
    pub geo_country: Option<&'a str>,
    pub billing_country: Option<&'a str>,
    pub vat_number: Option<&'a str>,
    pub amount_minor: i64,
    pub existing_provider_customer: bool,
    pub active_paid_customer: bool,
    pub recent_ip_checkouts: i64,
    pub recent_ip_workspaces: i64,
    pub recent_workspace_countries: i64,
    pub recent_payment_methods: i64,
    pub recent_payment_failures: i64,
    pub trusted_checkout_assessments: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckoutFraudPolicy {
    pub step_up_threshold: u8,
    pub manual_review_threshold: u8,
    pub block_threshold: u8,
}

impl Default for CheckoutFraudPolicy {
    fn default() -> Self {
        Self {
            step_up_threshold: 60,
            manual_review_threshold: 75,
            block_threshold: 90,
        }
    }
}

impl CheckoutFraudPolicy {
    pub const fn is_valid(self) -> bool {
        self.step_up_threshold <= self.manual_review_threshold
            && self.manual_review_threshold <= self.block_threshold
            && self.block_threshold <= 100
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckoutFraudAssessment {
    pub score: u8,
    pub decision: CheckoutFraudDecision,
    pub network_threat: NetworkThreatLevel,
    pub labels: Vec<String>,
    pub reasons: Vec<String>,
}

pub fn assess_checkout_fraud(input: CheckoutFraudInput<'_>) -> CheckoutFraudAssessment {
    let geo_country = normalize_country(input.geo_country);
    let billing_country = normalize_country(input.billing_country);
    let vat_country = vat_country(input.vat_number);
    let mut score = i16::from(input.network_risk_score.min(100));
    let mut labels = normalized_labels(input.network_labels);
    let mut reasons = Vec::new();

    push_unique(&mut labels, format!("network:{}", input.network_kind));
    push_unique(
        &mut labels,
        format!(
            "network_threat:{}",
            network_threat(input.network_risk_score).as_str()
        ),
    );
    push_network_reason(&mut reasons, input.network_kind, input.network_risk_score);

    if country_mismatch(geo_country.as_deref(), billing_country.as_deref()) {
        score += 25;
        push_unique(&mut labels, "mismatch:billing_country");
        push_unique(&mut reasons, "billing_country_mismatch");
    }
    if country_mismatch(geo_country.as_deref(), vat_country.as_deref()) {
        score += 20;
        push_unique(&mut labels, "mismatch:vat_country");
        push_unique(&mut reasons, "vat_country_mismatch");
    }
    if input.amount_minor >= 20_000 {
        score += 8;
        push_unique(&mut labels, "payment:high_amount");
        push_unique(&mut reasons, "high_amount_checkout");
    }
    if input.recent_ip_checkouts >= 5 {
        score += 15;
        push_unique(&mut labels, "velocity:ip_checkouts_high");
        push_unique(&mut reasons, "high_ip_checkout_velocity");
    }
    if input.recent_ip_workspaces >= 3 {
        score += 18;
        push_unique(&mut labels, "velocity:ip_workspace_reuse");
        push_unique(&mut reasons, "ip_reused_across_workspaces");
    }
    if input.recent_workspace_countries >= 3 {
        score += 15;
        push_unique(&mut labels, "velocity:workspace_country_changes");
        push_unique(&mut reasons, "workspace_country_velocity");
    }
    if input.recent_payment_methods >= 3 {
        score += 12;
        push_unique(&mut labels, "payment:multiple_recent_methods");
        push_unique(&mut reasons, "multiple_recent_payment_methods");
    }
    if input.recent_payment_failures >= 2 {
        score += 20;
        push_unique(&mut labels, "payment:recent_failures");
        push_unique(&mut reasons, "recent_payment_failures");
    }
    if input.trusted_checkout_assessments > 0 {
        score -= 20;
        push_unique(&mut labels, "customer:trusted_fraud_review");
    }
    if input.existing_provider_customer {
        score -= 10;
        push_unique(&mut labels, "customer:known_provider_customer");
    } else {
        score += 8;
        push_unique(&mut labels, "customer:new_provider_customer");
        push_unique(&mut reasons, "new_provider_customer");
    }
    if input.active_paid_customer {
        score -= 25;
        push_unique(&mut labels, "customer:active_paid");
    }

    let score = score.clamp(0, 100) as u8;
    CheckoutFraudAssessment {
        score,
        decision: checkout_fraud_decision(score, input.policy),
        network_threat: network_threat(input.network_risk_score),
        labels,
        reasons,
    }
}

pub fn checkout_fraud_enforcement_action(
    enabled: bool,
    decision: CheckoutFraudDecision,
) -> CheckoutFraudEnforcementAction {
    if !enabled {
        return CheckoutFraudEnforcementAction::Observe;
    }
    match decision {
        CheckoutFraudDecision::Allow => CheckoutFraudEnforcementAction::Allow,
        CheckoutFraudDecision::Monitor => CheckoutFraudEnforcementAction::Monitor,
        CheckoutFraudDecision::StepUp => CheckoutFraudEnforcementAction::StepUp,
        CheckoutFraudDecision::ManualReview => CheckoutFraudEnforcementAction::ManualReviewHold,
        CheckoutFraudDecision::Block => CheckoutFraudEnforcementAction::Block,
    }
}

fn push_network_reason(reasons: &mut Vec<String>, network_kind: &str, score: u8) {
    match network_kind {
        "tor" => push_unique(reasons, "tor_network"),
        "proxy" => push_unique(reasons, "proxy_network"),
        "vpn" => push_unique(reasons, "vpn_network"),
        "datacenter" => push_unique(reasons, "datacenter_network"),
        _ if score >= 80 => push_unique(reasons, "high_network_risk"),
        _ if score >= 60 => push_unique(reasons, "elevated_network_risk"),
        _ => {}
    }
}

fn checkout_fraud_decision(score: u8, policy: CheckoutFraudPolicy) -> CheckoutFraudDecision {
    if score >= policy.block_threshold {
        CheckoutFraudDecision::Block
    } else if score >= policy.manual_review_threshold {
        CheckoutFraudDecision::ManualReview
    } else if score >= policy.step_up_threshold {
        CheckoutFraudDecision::StepUp
    } else if score >= 40 {
        CheckoutFraudDecision::Monitor
    } else {
        CheckoutFraudDecision::Allow
    }
}

fn network_threat(score: u8) -> NetworkThreatLevel {
    match score {
        0..=39 => NetworkThreatLevel::Low,
        40..=59 => NetworkThreatLevel::Elevated,
        60..=79 => NetworkThreatLevel::High,
        _ => NetworkThreatLevel::Critical,
    }
}

fn country_mismatch(left: Option<&str>, right: Option<&str>) -> bool {
    matches!((left, right), (Some(left), Some(right)) if left != right)
}

fn normalize_country(country: Option<&str>) -> Option<String> {
    country
        .map(str::trim)
        .filter(|value| value.len() == 2)
        .map(str::to_ascii_uppercase)
}

fn vat_country(vat_number: Option<&str>) -> Option<String> {
    let vat = vat_number?.trim();
    let prefix = vat.get(0..2)?;
    prefix
        .chars()
        .all(|character| character.is_ascii_alphabetic())
        .then(|| prefix.to_ascii_uppercase())
}

fn normalized_labels(labels: &[String]) -> Vec<String> {
    let mut output = Vec::new();
    for label in labels {
        let label = label.trim().replace([' ', '-'], "_").to_ascii_lowercase();
        if !label.is_empty() {
            push_unique(&mut output, label);
        }
    }
    output
}

fn push_unique(labels: &mut Vec<String>, label: impl Into<String>) {
    let label = label.into();
    if !labels.iter().any(|item| item == &label) {
        labels.push(label);
    }
}

#[cfg(test)]
#[path = "fraud.tests.rs"]
mod tests;
