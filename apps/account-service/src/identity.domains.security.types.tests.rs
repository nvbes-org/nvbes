use super::{SecurityEventView, SecurityEventsResponse, SecurityEventsSummary};
use chrono::{TimeZone, Utc};
use uuid::Uuid;

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
        next_cursor: Some("opaque-cursor".to_string()),
        has_more: true,
        summary: SecurityEventsSummary {
            total_events: 1,
            ..SecurityEventsSummary::default()
        },
    })
    .expect("serializes");

    assert_eq!(payload["events"][0]["geo_country_code"], "FR");
    assert_eq!(payload["events"][0]["geo_network_kind"], "residential");
    assert_eq!(payload["summary"]["total_events"], 1);
    assert_eq!(payload["next_cursor"], "opaque-cursor");
    assert_eq!(payload["has_more"], true);
}
