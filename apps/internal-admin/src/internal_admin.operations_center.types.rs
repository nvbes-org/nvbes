use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub(crate) struct OperationsActionResult {
    pub(crate) object_id: Uuid,
    pub(crate) action_kind: &'static str,
    pub(crate) status: &'static str,
    pub(crate) audit_action: &'static str,
}

pub(crate) fn action_result(
    object_id: Uuid,
    action_kind: &'static str,
    status: &'static str,
    audit_action: &'static str,
) -> OperationsActionResult {
    OperationsActionResult {
        object_id,
        action_kind,
        status,
        audit_action,
    }
}
