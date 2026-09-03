use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::cockpit_degraded::DegradedScenario;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "action_type", content = "payload", rename_all = "snake_case")]
pub enum OperatorActionPayload {
    ReplayEmailMessage {
        message_id: Uuid,
    },
    ApplyEmailSuppression {
        email: String,
        reason: String,
    },
    ReleaseEmailSuppression {
        email: String,
    },
    ResolveTrustRiskCase {
        case_id: Uuid,
        resolution: String,
    },
    ReconcileBillingEvent {
        event_id: String,
        resolution: String,
    },
    ActivateDegradedProcedure {
        scenario: DegradedScenario,
    },
    DeactivateDegradedProcedure {
        scenario: DegradedScenario,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OperatorCommand {
    pub operator_id: String,
    pub reason: String,
    pub idempotency_key: String,
    pub correlation_id: Uuid,
    pub payload: OperatorActionPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OperatorActionReceipt {
    pub receipt_id: Uuid,
    pub operator_id: String,
    pub action_type: String,
    pub idempotency_key: String,
    pub correlation_id: Uuid,
    pub executed_at: DateTime<Utc>,
    pub status: String,
    pub details: String,
    pub is_reversible: bool,
    pub reversal_instruction: Option<String>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ActionExecutionError {
    #[error("operator_id must be at least 3 characters")]
    InvalidOperatorId,
    #[error("reason is mandatory and must be between 3 and 300 characters (received {0} chars)")]
    InvalidReason(usize),
    #[error("idempotency_key is mandatory and cannot be empty")]
    MissingIdempotencyKey,
    #[error("action execution failed: {0}")]
    ExecutionFailed(String),
}

pub struct OperatorActionDispatcher;

impl OperatorActionDispatcher {
    pub fn validate_command(cmd: &OperatorCommand) -> Result<(), ActionExecutionError> {
        if cmd.operator_id.trim().len() < 3 {
            return Err(ActionExecutionError::InvalidOperatorId);
        }
        let reason_len = cmd.reason.trim().len();
        if reason_len < 3 || cmd.reason.len() > 300 {
            return Err(ActionExecutionError::InvalidReason(reason_len));
        }
        if cmd.idempotency_key.trim().is_empty() {
            return Err(ActionExecutionError::MissingIdempotencyKey);
        }
        Ok(())
    }

    pub fn execute(cmd: OperatorCommand) -> Result<OperatorActionReceipt, ActionExecutionError> {
        Self::validate_command(&cmd)?;

        let (action_type, details, is_reversible, reversal_instruction) = match &cmd.payload {
            OperatorActionPayload::ReplayEmailMessage { message_id } => (
                "replay_email_message".to_string(),
                format!("Dispatched manual replay for message {}", message_id),
                false,
                None,
            ),
            OperatorActionPayload::ApplyEmailSuppression { email, reason } => (
                "apply_email_suppression".to_string(),
                format!("Suppression applied to {}: {}", email, reason),
                true,
                Some(format!("Execute release_email_suppression for {}", email)),
            ),
            OperatorActionPayload::ReleaseEmailSuppression { email } => (
                "release_email_suppression".to_string(),
                format!("Suppression released for {}", email),
                true,
                Some(format!("Execute apply_email_suppression for {}", email)),
            ),
            OperatorActionPayload::ResolveTrustRiskCase {
                case_id,
                resolution,
            } => (
                "resolve_trust_risk_case".to_string(),
                format!("Case {} marked resolved: {}", case_id, resolution),
                true,
                Some(format!(
                    "Reopen review case {} via trust-risk operations",
                    case_id
                )),
            ),
            OperatorActionPayload::ReconcileBillingEvent {
                event_id,
                resolution,
            } => (
                "reconcile_billing_event".to_string(),
                format!("Event {} manually reconciled: {}", event_id, resolution),
                false,
                None,
            ),
            OperatorActionPayload::ActivateDegradedProcedure { scenario } => (
                "activate_degraded_procedure".to_string(),
                format!("Activated degraded mode procedure for {:?}", scenario),
                true,
                Some(format!(
                    "Execute deactivate_degraded_procedure for {:?}",
                    scenario
                )),
            ),
            OperatorActionPayload::DeactivateDegradedProcedure { scenario } => (
                "deactivate_degraded_procedure".to_string(),
                format!("Deactivated degraded mode procedure for {:?}", scenario),
                true,
                Some(format!(
                    "Execute activate_degraded_procedure for {:?}",
                    scenario
                )),
            ),
        };

        Ok(OperatorActionReceipt {
            receipt_id: Uuid::new_v4(),
            operator_id: cmd.operator_id,
            action_type,
            idempotency_key: cmd.idempotency_key,
            correlation_id: cmd.correlation_id,
            executed_at: Utc::now(),
            status: "executed".to_string(),
            details,
            is_reversible,
            reversal_instruction,
        })
    }
}

#[cfg(test)]
#[path = "platform.cockpit.actions.tests.rs"]
mod tests;
