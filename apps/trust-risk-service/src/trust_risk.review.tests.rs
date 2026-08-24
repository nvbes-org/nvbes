use super::ReviewState;

#[test]
fn review_state_machine_is_explicit_and_resolution_requires_a_label() {
    assert!(ReviewState::Open.permits(ReviewState::InReview, false));
    assert!(!ReviewState::Open.permits(ReviewState::Resolved, true));
    assert!(!ReviewState::InReview.permits(ReviewState::Resolved, false));
    assert!(ReviewState::InReview.permits(ReviewState::Resolved, true));
    assert!(ReviewState::InReview.permits(ReviewState::Inconclusive, false));
    assert!(!ReviewState::Resolved.permits(ReviewState::Open, true));
}
