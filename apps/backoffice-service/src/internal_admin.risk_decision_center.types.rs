use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub(crate) struct RiskActionResult {
    pub(crate) object_id: Uuid,
    pub(crate) action_kind: String,
    pub(crate) status: String,
    pub(crate) audit_action: String,
}

pub(crate) fn action_result(
    object_id: Uuid,
    action_kind: impl Into<String>,
    status: impl Into<String>,
    audit_action: impl Into<String>,
) -> RiskActionResult {
    RiskActionResult {
        object_id,
        action_kind: action_kind.into(),
        status: status.into(),
        audit_action: audit_action.into(),
    }
}
