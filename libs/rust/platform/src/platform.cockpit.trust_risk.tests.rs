use super::*;
use crate::cockpit_model::TrustRiskCaseItem;

fn case(recommendation: &str, state: &str) -> TrustRiskCaseItem {
    TrustRiskCaseItem {
        case_id: Uuid::new_v4(),
        evaluation_id: Uuid::new_v4(),
        score: 42,
        band: "elevated".to_string(),
        recommendation: recommendation.to_string(),
        state: state.to_string(),
        created_at: Utc::now(),
    }
}

#[test]
fn enforces_shadow_mode_and_blocks_automated_enforcement() {
    for recommendation in [
        RiskRecommendation::Allow,
        RiskRecommendation::Challenge,
        RiskRecommendation::Review,
        RiskRecommendation::Deny,
    ] {
        assert_eq!(
            TrustRiskCockpitView::prevent_automatic_enforcement(recommendation),
            Err(TrustRiskReviewError::AutomatedEnforcementForbidden(
                recommendation
            ))
        );
    }
}

#[test]
fn summarizes_all_recommendations_and_pending_states() {
    let cases = [
        case("ALLOW", "open"),
        case("Challenge", "in_review"),
        case("review", "closed"),
        case("deny", "resolved"),
        case("unknown-signal", "open"),
    ];

    let summary = TrustRiskCockpitView::summarize(&cases);
    assert!(summary.shadow_mode);
    assert_eq!(summary.allow_count, 1);
    assert_eq!(summary.challenge_count, 1);
    assert_eq!(summary.review_count, 1);
    assert_eq!(summary.deny_count, 1);
    assert_eq!(summary.pending_reviews_count, 3);
    assert_eq!(summary.active_cases.len(), 5);
}

#[test]
fn case_already_resolved_error_is_distinct() {
    let case_id = Uuid::new_v4();
    let err = TrustRiskReviewError::CaseAlreadyResolved(case_id);
    assert_eq!(
        err.to_string(),
        format!("review case {case_id} is already resolved")
    );
}
