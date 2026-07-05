use super::*;
use chrono::Duration;

use crate::domains::enterprise::access_reviews::types::{
    AccessReviewCampaignScopeInput, AccessReviewChangeInput,
};

#[test]
fn campaign_validation_requires_a_future_due_date() {
    let err = validate_campaign_input(&campaign_input(Utc::now() - Duration::minutes(1)))
        .expect_err("past due dates must be rejected");

    assert_eq!(err.code, "validation_failed");
}

#[test]
fn campaign_validation_requires_at_least_one_scope() {
    let mut input = campaign_input(Utc::now() + Duration::days(1));
    input.scope = AccessReviewCampaignScopeInput {
        include_members: false,
        include_roles: false,
        include_service_accounts: false,
        include_oauth_clients: false,
    };

    let err = validate_campaign_input(&input).expect_err("empty review scopes must be rejected");

    assert_eq!(err.code, "validation_failed");
}

#[test]
fn campaign_validation_accepts_trimmed_names_and_any_scope() {
    let result = validate_campaign_input(&campaign_input(Utc::now() + Duration::days(1)));

    assert!(result.is_ok());
}

#[test]
fn schedule_validation_bounds_periods() {
    let mut input = schedule_input();
    input.recurrence_days = 6;

    let err = validate_schedule_input(&input).expect_err("short periods are rejected");

    assert_eq!(err.code, "validation_failed");
}

#[test]
fn schedule_validation_rejects_due_after_recurrence() {
    let mut input = schedule_input();
    input.recurrence_days = 30;
    input.due_after_days = 31;

    let err = validate_schedule_input(&input).expect_err("due window must fit period");

    assert_eq!(err.code, "validation_failed");
}

#[test]
fn decision_validation_rejects_pending_decision() {
    let err = validate_decision_input(&decision_input(AccessReviewItemDecision::Pending, None))
        .expect_err("pending is not a terminal reviewer decision");

    assert_eq!(err.code, "validation_failed");
}

#[test]
fn decision_validation_requires_note_for_remediation_decisions() {
    for decision in [
        AccessReviewItemDecision::Revoked,
        AccessReviewItemDecision::Changed,
    ] {
        let err = validate_decision_input(&decision_input(decision, Some("   ")))
            .expect_err("remediation decisions need reviewer rationale");

        assert_eq!(err.code, "validation_failed");
    }
}

#[test]
fn decision_validation_allows_approval_without_note() {
    let result = validate_decision_input(&decision_input(AccessReviewItemDecision::Approved, None));

    assert!(result.is_ok());
}

#[test]
fn decision_validation_rejects_overlong_note() {
    let err = validate_decision_input(&decision_input(
        AccessReviewItemDecision::Approved,
        Some(&"x".repeat(1001)),
    ))
    .expect_err("long reviewer notes must be bounded");

    assert_eq!(err.code, "validation_failed");
}

#[test]
fn decision_validation_requires_target_role_for_change_decision() {
    let err = validate_decision_input(&AccessReviewDecisionInput {
        decision: AccessReviewItemDecision::Changed,
        note: Some("Reduce access".to_string()),
        change: None,
    })
    .expect_err("change decisions need explicit target state");

    assert_eq!(err.code, "validation_failed");
}

#[test]
fn decision_validation_rejects_change_payload_for_other_decisions() {
    let err = validate_decision_input(&AccessReviewDecisionInput {
        decision: AccessReviewItemDecision::Approved,
        note: None,
        change: Some(AccessReviewChangeInput {
            target_role: Some("viewer".to_string()),
        }),
    })
    .expect_err("change payload only belongs to change decisions");

    assert_eq!(err.code, "validation_failed");
}

#[test]
fn decision_validation_accepts_change_with_target_role_and_note() {
    let result = validate_decision_input(&AccessReviewDecisionInput {
        decision: AccessReviewItemDecision::Changed,
        note: Some("Reduce to viewer".to_string()),
        change: Some(AccessReviewChangeInput {
            target_role: Some("viewer".to_string()),
        }),
    });

    assert!(result.is_ok());
}

#[test]
fn decision_database_values_are_stable() {
    assert_eq!(
        decision_as_db(&AccessReviewItemDecision::Approved),
        "approved"
    );
    assert_eq!(
        decision_as_db(&AccessReviewItemDecision::Revoked),
        "revoked"
    );
    assert_eq!(
        decision_as_db(&AccessReviewItemDecision::Changed),
        "changed"
    );
    assert_eq!(
        decision_as_db(&AccessReviewItemDecision::Pending),
        "pending"
    );
}

fn campaign_input(due_at: chrono::DateTime<Utc>) -> CreateAccessReviewCampaignInput {
    CreateAccessReviewCampaignInput {
        name: " Q2 access review ".to_string(),
        description: None,
        due_at,
        scope: AccessReviewCampaignScopeInput {
            include_members: true,
            include_roles: false,
            include_service_accounts: false,
            include_oauth_clients: false,
        },
    }
}

fn decision_input(
    decision: AccessReviewItemDecision,
    note: Option<&str>,
) -> AccessReviewDecisionInput {
    AccessReviewDecisionInput {
        decision,
        note: note.map(str::to_string),
        change: None,
    }
}

fn schedule_input() -> CreateAccessReviewScheduleInput {
    CreateAccessReviewScheduleInput {
        name: "Quarterly review".to_string(),
        description: None,
        recurrence_days: 90,
        due_after_days: 14,
        scope: AccessReviewCampaignScopeInput {
            include_members: true,
            include_roles: true,
            include_service_accounts: true,
            include_oauth_clients: true,
        },
    }
}
