use chrono::Utc;
use uuid::Uuid;

use super::{AuditEventView, events_to_csv};

#[test]
fn events_to_csv_includes_actor_principal_id_column() {
    let csv = events_to_csv(&[AuditEventView {
        id: Uuid::new_v4(),
        workspace_id: Uuid::new_v4(),
        actor_user_id: Some(Uuid::new_v4()),
        actor_principal_id: Some(Uuid::new_v4()),
        actor_email: Some("user@example.com".to_string()),
        action: "audit.test".to_string(),
        target_type: "workspace".to_string(),
        target_id: Some(Uuid::new_v4()),
        ip: Some("127.0.0.1".to_string()),
        user_agent: Some("test-agent".to_string()),
        geo_country_code: Some("FR".to_string()),
        geo_network_kind: Some("vpn".to_string()),
        geo_risk_score: Some(90),
        geo_risk_labels: vec!["vpn".to_string()],
        network_block_reason: Some("vpn".to_string()),
        metadata: serde_json::json!({"ok": true}),
        previous_event_hash: Some("prev".to_string()),
        event_hash: "hash".to_string(),
        created_at: Utc::now(),
    }]);

    let header = csv.lines().next().expect("csv header");
    assert!(header.contains("actor_principal_id"));
    assert!(header.contains("geo_network_kind"));
    assert!(header.contains("network_block_reason"));
    assert!(csv.contains("vpn"));
    assert!(csv.contains("user@example.com"));
}
