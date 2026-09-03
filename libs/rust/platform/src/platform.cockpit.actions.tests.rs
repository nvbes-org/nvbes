use super::*;

#[test]
fn executes_valid_operator_command_with_receipt() {
    let cmd = OperatorCommand {
        operator_id: "operator-01".to_string(),
        reason: "Valid incident mitigation reason".to_string(),
        idempotency_key: "action-key-123".to_string(),
        correlation_id: Uuid::new_v4(),
        payload: OperatorActionPayload::ApplyEmailSuppression {
            email: "bot@spammer.org".to_string(),
            reason: "Confirmed spam sender".to_string(),
        },
    };

    let receipt = OperatorActionDispatcher::execute(cmd).expect("command must succeed");
    assert_eq!(receipt.status, "executed");
    assert_eq!(receipt.operator_id, "operator-01");
    assert!(receipt.is_reversible);
    assert!(receipt.reversal_instruction.is_some());
}

#[test]
fn rejects_missing_or_short_reason() {
    let cmd = OperatorCommand {
        operator_id: "operator-01".to_string(),
        reason: "no".to_string(),
        idempotency_key: "action-key-123".to_string(),
        correlation_id: Uuid::new_v4(),
        payload: OperatorActionPayload::ReleaseEmailSuppression {
            email: "user@example.com".to_string(),
        },
    };

    assert_eq!(
        OperatorActionDispatcher::execute(cmd),
        Err(ActionExecutionError::InvalidReason(2))
    );
}

#[test]
fn rejects_missing_idempotency_key() {
    let cmd = OperatorCommand {
        operator_id: "operator-01".to_string(),
        reason: "Valid reason for action".to_string(),
        idempotency_key: "   ".to_string(),
        correlation_id: Uuid::new_v4(),
        payload: OperatorActionPayload::ReleaseEmailSuppression {
            email: "user@example.com".to_string(),
        },
    };

    assert_eq!(
        OperatorActionDispatcher::execute(cmd),
        Err(ActionExecutionError::MissingIdempotencyKey)
    );
}
