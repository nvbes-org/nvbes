use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::cockpit_model::{TrustRiskCaseItem, TrustRiskSummary};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskBand {
    Low,
    Elevated,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskRecommendation {
    Allow,
    Challenge,
    Review,
    Deny,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DetailedEvaluationView {
    pub evaluation_id: Uuid,
    pub score: i16,
    pub band: RiskBand,
    pub recommendation: RiskRecommendation,
    pub reasons: Vec<String>,
    pub evaluated_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub requires_human_decision: bool,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TrustRiskReviewError {
    #[error("cannot automatically apply recommendation {0:?}: automated enforcement forbidden in V1")]
    AutomatedEnforcementForbidden(RiskRecommendation),
    #[error("review case {0} is already resolved")]
    CaseAlreadyResolved(Uuid),
}

pub struct TrustRiskCockpitView;

impl TrustRiskCockpitView {
    pub fn summarize(cases: &[TrustRiskCaseItem]) -> TrustRiskSummary {
        let mut allow_count = 0;
        let mut challenge_count = 0;
        let mut review_count = 0;
        let mut deny_count = 0;
        let mut pending_reviews = 0;

        for case in cases {
            match case.recommendation.to_ascii_lowercase().as_str() {
                "allow" => allow_count += 1,
                "challenge" => challenge_count += 1,
                "review" => review_count += 1,
                "deny" => deny_count += 1,
                _ => {}
            }
            if case.state == "open" || case.state == "in_review" {
                pending_reviews += 1;
            }
        }

        TrustRiskSummary {
            shadow_mode: true,
            pending_reviews_count: pending_reviews,
            allow_count,
            challenge_count,
            review_count,
            deny_count,
            active_cases: cases.to_vec(),
        }
    }

    pub fn prevent_automatic_enforcement(
        recommendation: RiskRecommendation,
    ) -> Result<(), TrustRiskReviewError> {
        Err(TrustRiskReviewError::AutomatedEnforcementForbidden(
            recommendation,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enforces_shadow_mode_and_blocks_automated_enforcement() {
        let err = TrustRiskCockpitView::prevent_automatic_enforcement(RiskRecommendation::Deny);
        assert_eq!(
            err,
            Err(TrustRiskReviewError::AutomatedEnforcementForbidden(
                RiskRecommendation::Deny
            ))
        );
    }

    #[test]
    fn summarizes_trust_risk_cases_correctly() {
        let case = TrustRiskCaseItem {
            case_id: Uuid::new_v4(),
            evaluation_id: Uuid::new_v4(),
            score: 85,
            band: "high".to_string(),
            recommendation: "deny".to_string(),
            state: "open".to_string(),
            created_at: Utc::now(),
        };

        let summary = TrustRiskCockpitView::summarize(&[case]);
        assert!(summary.shadow_mode);
        assert_eq!(summary.pending_reviews_count, 1);
        assert_eq!(summary.deny_count, 1);
        assert_eq!(summary.allow_count, 0);
    }
}
