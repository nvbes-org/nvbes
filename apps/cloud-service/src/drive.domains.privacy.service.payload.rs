use serde_json::json;
use uuid::Uuid;

use super::PrivacyRequestDraft;

pub(super) fn privacy_idempotency_key(draft: &PrivacyRequestDraft) -> String {
    let subject = draft
        .subject_user_id
        .or(draft.workspace_id)
        .map(|id| id.to_string())
        .unwrap_or_else(|| "unknown".to_owned());

    format!("privacy:{}:{subject}", draft.request_type)
}

pub(super) fn merge_request_payload(
    payload: serde_json::Value,
    request_id: Uuid,
) -> serde_json::Value {
    let mut payload = payload;
    if let Some(object) = payload.as_object_mut() {
        object.insert("privacy_request_id".to_owned(), json!(request_id));
    }
    payload
}

pub(super) fn privacy_request_payload(
    subject_user_id: Option<Uuid>,
    workspace_id: Option<Uuid>,
    requested_by: Uuid,
    requested_by_principal_id: Uuid,
) -> serde_json::Value {
    let mut payload = serde_json::Map::new();
    if let Some(subject_user_id) = subject_user_id {
        payload.insert("subject_user_id".to_owned(), json!(subject_user_id));
    }
    if let Some(workspace_id) = workspace_id {
        payload.insert("workspace_id".to_owned(), json!(workspace_id));
    }
    payload.insert("requested_by".to_owned(), json!(requested_by));
    payload.insert(
        "requested_by_principal_id".to_owned(),
        json!(requested_by_principal_id),
    );
    serde_json::Value::Object(payload)
}
