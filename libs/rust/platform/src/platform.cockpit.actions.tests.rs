use super::*;

#[test]
fn never_claims_an_unimplemented_domain_action_was_executed() {
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

    assert!(matches!(
        OperatorActionDispatcher::execute(cmd),
        Err(ActionExecutionError::ExecutionFailed(_))
    ));
}

#[test]
fn rejects_short_operator_id() {
    let cmd = OperatorCommand {
        operator_id: "ab".to_string(),
        reason: "Valid incident mitigation reason".to_string(),
        idempotency_key: "action-key-123".to_string(),
        correlation_id: Uuid::new_v4(),
        payload: OperatorActionPayload::ReleaseEmailSuppression {
            email: "user@example.com".to_string(),
        },
    };

    assert_eq!(
        OperatorActionDispatcher::execute(cmd),
        Err(ActionExecutionError::InvalidOperatorId)
    );
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
