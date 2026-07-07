use super::{PrivacyRequestResponse, payload::privacy_request_payload};
use uuid::Uuid;

#[test]
fn privacy_request_payload_includes_requested_by_principal_id() {
    let payload = privacy_request_payload(
        Some(Uuid::nil()),
        Some(Uuid::new_v4()),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );

    let object = payload.as_object().expect("object payload");
    assert!(object.contains_key("requested_by_principal_id"));
    assert!(object.contains_key("requested_by"));
}

#[test]
fn privacy_request_response_includes_requested_by_principal_id() {
    let response = PrivacyRequestResponse {
        request_id: Uuid::new_v4(),
        request_type: "workspace_export".to_string(),
        status: "queued".to_string(),
        worker_job_id: Uuid::new_v4(),
        requested_by_principal_id: Uuid::new_v4(),
        message: "Queued".to_string(),
        guardrails: vec!["guardrail".to_string()],
        requested_at: chrono::Utc::now(),
    };

    let json = serde_json::to_value(response).expect("serializable response");
    assert!(json.get("requested_by_principal_id").is_some());
}
