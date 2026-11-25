use crate::operations_error::OperationsError;
use crate::operations_model::{Action, CaseStatus, Command};
use chrono::Datelike;

pub fn text(value: &str, min: usize, max: usize) -> Result<(), OperationsError> {
    if !(min..=max).contains(&value.trim().chars().count()) || value.contains('\0') {
        return Err(OperationsError::Invalid(
            "text length outside permitted bounds",
        ));
    }
    Ok(())
}

pub fn validate(command: &Command) -> Result<(), OperationsError> {
    text(&command.reason, 3, 300)?;
    if command.idempotency_key.is_nil() || command.correlation_id.is_nil() {
        return Err(OperationsError::Invalid(
            "non-nil command identifiers required",
        ));
    }
    match &command.action {
        Action::OpenCase {
            subject_id,
            source,
            summary,
            ..
        } => {
            if subject_id.is_nil() {
                return Err(OperationsError::Invalid("subject required"));
            }
            text(source, 3, 200)?;
            text(summary, 3, 1000)?;
        }
        Action::Transition { evidence, .. } => text(evidence, 3, 500)?,
        Action::AddNote { note, evidence, .. } => {
            text(note, 3, 1000)?;
            text(evidence, 3, 500)?;
        }
        Action::RecordObservation {
            observed_at,
            api_reference,
            summary,
            ..
        } => {
            if *observed_at > chrono::Utc::now() {
                return Err(OperationsError::Invalid(
                    "observation cannot be in the future",
                ));
            }
            text(api_reference, 3, 500)?;
            text(summary, 3, 1000)?;
        }
        Action::RecordCost {
            month,
            provider,
            category,
            actual_cents,
            forecast_cents,
            evidence,
            ..
        } => {
            if month.day() != 1 || !(2026..=2100).contains(&month.year()) {
                return Err(OperationsError::Invalid(
                    "month must be the first day of a supported month",
                ));
            }
            if forecast_cents < actual_cents {
                return Err(OperationsError::Invalid(
                    "forecast cannot be below actual spend",
                ));
            }
            text(provider, 2, 100)?;
            text(category, 2, 100)?;
            text(evidence, 3, 500)?;
        }
    }
    Ok(())
}

pub fn transition(from: CaseStatus, to: CaseStatus) -> Result<(), OperationsError> {
    use CaseStatus::*;
    let allowed = matches!(
        (from, to),
        (Open, Investigating)
            | (Investigating, AwaitingUser | ActionPending | Resolved)
            | (AwaitingUser, Investigating)
            | (ActionPending, Investigating)
            | (Resolved, Closed | Appealed)
            | (Closed, Appealed)
            | (Appealed, Investigating)
    );
    if !allowed {
        return Err(OperationsError::Invalid("invalid case transition"));
    }
    Ok(())
}

#[cfg(test)]
#[path = "platform.operations.validation.tests.rs"]
mod tests;
