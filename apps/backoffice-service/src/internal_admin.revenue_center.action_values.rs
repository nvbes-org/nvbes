use crate::error::AppError;

pub(crate) fn action_kind(value: &str) -> Result<&'static str, AppError> {
    match value {
        "close_dunning_case" => Ok("close_dunning_case"),
        "reopen_dunning_case" => Ok("reopen_dunning_case"),
        "hold_invoice" => Ok("hold_invoice"),
        "release_invoice" => Ok("release_invoice"),
        "review_dispute" => Ok("review_dispute"),
        "resolve_dispute" => Ok("resolve_dispute"),
        _ => Err(AppError::internal(
            "billing_grpc_invalid_action_kind",
            format!("unexpected revenue action kind: {value}"),
        )),
    }
}

pub(crate) fn action_status(value: &str) -> Result<&'static str, AppError> {
    match value {
        "closed" => Ok("closed"),
        "open" => Ok("open"),
        "held" => Ok("held"),
        "released" => Ok("released"),
        "under_review" => Ok("under_review"),
        "resolved" => Ok("resolved"),
        _ => Err(AppError::internal(
            "billing_grpc_invalid_action_status",
            format!("unexpected revenue action status: {value}"),
        )),
    }
}

pub(crate) fn audit_action(value: &str) -> Result<&'static str, AppError> {
    match value {
        "revenue.dunning_case.closed" => Ok("revenue.dunning_case.closed"),
        "revenue.dunning_case.reopened" => Ok("revenue.dunning_case.reopened"),
        "revenue.invoice.held" => Ok("revenue.invoice.held"),
        "revenue.invoice.released" => Ok("revenue.invoice.released"),
        "revenue.dispute.reviewed" => Ok("revenue.dispute.reviewed"),
        "revenue.dispute.resolved" => Ok("revenue.dispute.resolved"),
        _ => Err(AppError::internal(
            "billing_grpc_invalid_audit_action",
            format!("unexpected revenue audit action: {value}"),
        )),
    }
}

pub(crate) fn target_type(value: &str) -> Result<&'static str, AppError> {
    match value {
        "billing_dunning_case" => Ok("billing_dunning_case"),
        "billing_invoice" => Ok("billing_invoice"),
        "billing_dispute" => Ok("billing_dispute"),
        _ => Err(AppError::internal(
            "billing_grpc_invalid_target_type",
            format!("unexpected revenue target type: {value}"),
        )),
    }
}
