use super::*;
use crate::cockpit_model::ServiceId;
use crate::operations_model::{Action, CaseCategory, CaseStatus, Command};
use chrono::Utc;
use uuid::Uuid;

fn base_command(action: Action) -> Command {
    Command {
        idempotency_key: Uuid::new_v4(),
        correlation_id: Uuid::new_v4(),
        reason: "Valid operator reason".into(),
        action,
    }
}

#[test]
fn text_rejects_bounds_and_nul() {
    assert!(text("abc", 3, 10).is_ok());
    assert!(text("  ab  ", 3, 10).is_err());
    assert!(text("too-long-value", 1, 5).is_err());
    assert!(text("bad\0text", 1, 20).is_err());
}

#[test]
fn validate_rejects_nil_identifiers() {
    let mut command = base_command(Action::OpenCase {
        category: CaseCategory::Support,
        owner: ServiceId::Account,
        subject_id: Uuid::new_v4(),
        source: "ticket".into(),
        summary: "Need help with account access".into(),
        related_case_id: None,
    });
    command.idempotency_key = Uuid::nil();
    assert!(matches!(
        validate(&command),
        Err(OperationsError::Invalid(_))
    ));
}

#[test]
fn validate_open_case_requires_subject_and_text() {
    let command = base_command(Action::OpenCase {
        category: CaseCategory::Support,
        owner: ServiceId::Account,
        subject_id: Uuid::nil(),
        source: "ticket".into(),
        summary: "Need help with account access".into(),
        related_case_id: None,
    });
    assert!(matches!(
        validate(&command),
        Err(OperationsError::Invalid(_))
    ));
}

#[test]
fn validate_add_note_and_observation_branches() {
    let note = base_command(Action::AddNote {
        case_id: Uuid::new_v4(),
        expected_version: 1,
        note: "ok".into(),
        evidence: "API receipt verified".into(),
    });
    assert!(matches!(validate(&note), Err(OperationsError::Invalid(_))));

    let future = base_command(Action::RecordObservation {
        case_id: Uuid::new_v4(),
        expected_version: 1,
        service: ServiceId::Email,
        observed_at: Utc::now() + chrono::Duration::hours(2),
        api_reference: "Email operations receipt".into(),
        summary: "Delivery investigated without copying content".into(),
    });
    assert!(matches!(
        validate(&future),
        Err(OperationsError::Invalid(_))
    ));
}

#[test]
fn validate_cost_month_and_forecast_rules() {
    let bad_day = base_command(Action::RecordCost {
        month: chrono::NaiveDate::from_ymd_opt(2026, 9, 15).unwrap(),
        provider: "Scaleway".into(),
        category: "compute".into(),
        actual_cents: 100,
        forecast_cents: 200,
        evidence: "invoice reference".into(),
        replaces: None,
    });
    assert!(matches!(
        validate(&bad_day),
        Err(OperationsError::Invalid(_))
    ));

    let forecast_below = base_command(Action::RecordCost {
        month: chrono::NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
        provider: "Scaleway".into(),
        category: "compute".into(),
        actual_cents: 500,
        forecast_cents: 100,
        evidence: "invoice reference".into(),
        replaces: None,
    });
    assert!(matches!(
        validate(&forecast_below),
        Err(OperationsError::Invalid(_))
    ));
}

#[test]
fn transition_matrix_rejects_illegal_pairs() {
    assert!(transition(CaseStatus::Open, CaseStatus::Investigating).is_ok());
    assert!(matches!(
        transition(CaseStatus::Open, CaseStatus::Closed),
        Err(OperationsError::Invalid(_))
    ));
    assert!(matches!(
        transition(CaseStatus::Resolved, CaseStatus::Open),
        Err(OperationsError::Invalid(_))
    ));
}
