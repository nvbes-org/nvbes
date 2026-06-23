use super::{
    ListRecoveryReviewsResponse, RecoveryReviewView, SecurityEventView, SecurityEventsResponse,
    SecurityEventsSummary, WorkerQueueStatusResponse, WorkerQueueStatusView,
};
use chrono::{TimeZone, Utc};
use uuid::Uuid;

#[test]
fn recovery_reviews_response_serializes_expected_contract() {
    let request_id = Uuid::from_u128(11);
    let principal_id = Uuid::from_u128(12);
    let approved_by_principal_id = Uuid::from_u128(13);
    let secondary_approved_by_principal_id = Uuid::from_u128(14);
    let available_at = Utc
        .with_ymd_and_hms(2026, 5, 12, 8, 0, 0)
        .single()
        .expect("valid timestamp");
    let approved_at = Utc
        .with_ymd_and_hms(2026, 5, 12, 8, 15, 0)
        .single()
        .expect("valid timestamp");
    let review_available_at = Utc
        .with_ymd_and_hms(2026, 5, 12, 9, 0, 0)
        .single()
        .expect("valid timestamp");
    let secondary_approved_at = Utc
        .with_ymd_and_hms(2026, 5, 12, 9, 30, 0)
        .single()
        .expect("valid timestamp");
    let created_at = Utc
        .with_ymd_and_hms(2026, 5, 12, 8, 30, 0)
        .single()
        .expect("valid timestamp");
    let updated_at = Utc
        .with_ymd_and_hms(2026, 5, 12, 9, 45, 0)
        .single()
        .expect("valid timestamp");

    let response = ListRecoveryReviewsResponse {
        workspace_id: Uuid::from_u128(15),
        requests: vec![RecoveryReviewView {
            request_id,
            principal_id,
            email: "enterprise@example.com".to_string(),
            status: "first_approved".to_string(),
            available_at,
            approved_by_principal_id: Some(approved_by_principal_id),
            approved_at: Some(approved_at),
            review_available_at: Some(review_available_at),
            secondary_approved_by_principal_id: Some(secondary_approved_by_principal_id),
            secondary_approved_at: Some(secondary_approved_at),
            created_at,
            updated_at,
        }],
    };

    let payload = serde_json::to_value(response).expect("serializes");

    assert_eq!(payload["requests"][0]["request_id"], request_id.to_string());
    assert_eq!(
        payload["requests"][0]["principal_id"],
        principal_id.to_string()
    );
    assert_eq!(payload["requests"][0]["status"], "first_approved");
    assert_eq!(
        payload["requests"][0]["review_available_at"],
        "2026-05-12T09:00:00Z"
    );
    assert_eq!(
        payload["requests"][0]["secondary_approved_at"],
        "2026-05-12T09:30:00Z"
    );
}

#[test]
fn security_events_response_serializes_summary_and_geo_fields() {
    let event_id = Uuid::from_u128(21);
    let created_at = Utc
        .with_ymd_and_hms(2026, 6, 23, 12, 0, 0)
        .single()
        .expect("valid timestamp");

    let payload = serde_json::to_value(SecurityEventsResponse {
        events: vec![SecurityEventView {
            id: event_id,
            event_type: "login_success".to_string(),
            created_at,
            ip_address: Some("203.0.113.42".to_string()),
            user_agent: Some("test-agent".to_string()),
            status: Some("allow".to_string()),
            risk_score: Some(15.0),
            geo_country_code: Some("FR".to_string()),
            geo_source: Some("personal_database".to_string()),
            geo_confidence: Some("high".to_string()),
            geo_network_kind: Some("residential".to_string()),
            geo_risk_score: Some(15),
            geo_risk_labels: vec!["residential".to_string()],
        }],
        next_cursor: Some(event_id),
        summary: SecurityEventsSummary {
            total_events: 1,
            ..SecurityEventsSummary::default()
        },
    })
    .expect("serializes");

    assert_eq!(payload["events"][0]["geo_country_code"], "FR");
    assert_eq!(payload["events"][0]["geo_network_kind"], "residential");
    assert_eq!(payload["summary"]["total_events"], 1);
}

#[test]
fn recovery_reviews_input_deserializes_cursor_pair() {
    let input: super::ListRecoveryReviewsInput = serde_json::from_value(serde_json::json!({
        "limit": 25,
        "before_created_at": "2026-05-12T09:00:00Z",
        "before_id": Uuid::from_u128(42)
    }))
    .expect("deserializes");

    assert_eq!(input.limit, Some(25));
    assert_eq!(
        input.before_created_at,
        Some(
            Utc.with_ymd_and_hms(2026, 5, 12, 9, 0, 0)
                .single()
                .expect("valid timestamp")
        )
    );
    assert_eq!(input.before_id, Some(Uuid::from_u128(42)));
}

#[test]
fn worker_queue_status_response_serializes_expected_contract() {
    let snapshot_at = Utc
        .with_ymd_and_hms(2026, 5, 12, 10, 0, 0)
        .single()
        .expect("valid timestamp");
    let response = WorkerQueueStatusResponse {
        workspace_id: Uuid::from_u128(99),
        queue_name: "billing.stripe.webhook.process".to_string(),
        snapshot_at,
        statuses: vec![WorkerQueueStatusView {
            status: "pending".to_string(),
            depth: 4,
            oldest_age_seconds: Some(32.5),
        }],
    };

    let payload = serde_json::to_value(response).expect("serializes");

    assert_eq!(payload["queue_name"], "billing.stripe.webhook.process");
    assert_eq!(payload["workspace_id"], Uuid::from_u128(99).to_string());
    assert_eq!(payload["snapshot_at"], "2026-05-12T10:00:00Z");
    assert_eq!(payload["statuses"][0]["status"], "pending");
    assert_eq!(payload["statuses"][0]["depth"], 4);
}
