use chrono::Utc;

use super::types::{
    AccessReviewDecisionInput, AccessReviewItemDecision, CloseAccessReviewCampaignInput,
    CreateAccessReviewCampaignInput, CreateAccessReviewScheduleInput,
};
use crate::http::error::AppError;

pub fn validate_decision_input(input: &AccessReviewDecisionInput) -> Result<(), AppError> {
    if matches!(input.decision, AccessReviewItemDecision::Pending) {
        return Err(AppError::bad_request(
            "validation_failed",
            "Review decision must approve, revoke, or change the item.",
        ));
    }
    if input
        .note
        .as_ref()
        .is_some_and(|note| note.trim().len() > 1000)
    {
        return Err(AppError::bad_request(
            "validation_failed",
            "Review note must be 1000 characters or fewer.",
        ));
    }
    if matches!(
        input.decision,
        AccessReviewItemDecision::Revoked | AccessReviewItemDecision::Changed
    ) && input
        .note
        .as_ref()
        .is_none_or(|note| note.trim().is_empty())
    {
        return Err(AppError::bad_request(
            "validation_failed",
            "A review note is required for revoke and change decisions.",
        ));
    }
    if matches!(input.decision, AccessReviewItemDecision::Changed) {
        let target_role = input
            .change
            .as_ref()
            .and_then(|change| change.target_role.as_deref())
            .map(str::trim)
            .ok_or_else(|| {
                AppError::bad_request(
                    "validation_failed",
                    "A target role is required for change decisions.",
                )
            })?;
        validate_workspace_role(target_role)?;
    } else if input.change.is_some() {
        return Err(AppError::bad_request(
            "validation_failed",
            "Change payload is only allowed for change decisions.",
        ));
    }
    Ok(())
}

pub fn validate_close_input(
    input: &CloseAccessReviewCampaignInput,
    pending_items: i64,
) -> Result<(), AppError> {
    if input
        .note
        .as_ref()
        .is_some_and(|note| note.trim().len() > 1000)
    {
        return Err(AppError::bad_request(
            "validation_failed",
            "Close note must be 1000 characters or fewer.",
        ));
    }
    if pending_items > 0
        && input
            .note
            .as_ref()
            .is_none_or(|note| note.trim().is_empty())
    {
        return Err(AppError::bad_request(
            "validation_failed",
            "A close note is required when closing a campaign with pending items.",
        ));
    }
    Ok(())
}

pub fn decision_as_db(decision: &AccessReviewItemDecision) -> &'static str {
    match decision {
        AccessReviewItemDecision::Approved => "approved",
        AccessReviewItemDecision::Revoked => "revoked",
        AccessReviewItemDecision::Changed => "changed",
        AccessReviewItemDecision::Pending => "pending",
    }
}

pub fn validate_campaign_input(input: &CreateAccessReviewCampaignInput) -> Result<(), AppError> {
    if input.name.trim().is_empty() {
        return Err(AppError::bad_request(
            "validation_failed",
            "Access review campaign name is required.",
        ));
    }
    if input.due_at <= Utc::now() {
        return Err(AppError::bad_request(
            "validation_failed",
            "Access review campaign due date must be in the future.",
        ));
    }
    if !has_scope(&input.scope) {
        return Err(AppError::bad_request(
            "validation_failed",
            "At least one access review scope must be selected.",
        ));
    }
    Ok(())
}

pub fn validate_schedule_input(input: &CreateAccessReviewScheduleInput) -> Result<(), AppError> {
    if input.name.trim().is_empty() {
        return Err(AppError::bad_request(
            "validation_failed",
            "Access review schedule name is required.",
        ));
    }
    if !(7..=366).contains(&input.recurrence_days) {
        return Err(AppError::bad_request(
            "validation_failed",
            "Access review recurrence must be between 7 and 366 days.",
        ));
    }
    if input.due_after_days < 1 || input.due_after_days > input.recurrence_days {
        return Err(AppError::bad_request(
            "validation_failed",
            "Access review due window must be at least one day and no longer than the recurrence.",
        ));
    }
    if !has_scope(&input.scope) {
        return Err(AppError::bad_request(
            "validation_failed",
            "At least one access review scope must be selected.",
        ));
    }
    Ok(())
}

pub fn validate_workspace_role(role: &str) -> Result<(), AppError> {
    match role.trim() {
        "owner" | "admin" | "member" | "viewer" => Ok(()),
        _ => Err(AppError::bad_request(
            "validation_failed",
            "Target role must be owner, admin, member, or viewer.",
        )),
    }
}

fn has_scope(scope: &super::types::AccessReviewCampaignScopeInput) -> bool {
    scope.include_members
        || scope.include_roles
        || scope.include_service_accounts
        || scope.include_oauth_clients
}

#[cfg(test)]
#[path = "identity.domains.enterprise.access_reviews.validation.tests.rs"]
mod tests;
