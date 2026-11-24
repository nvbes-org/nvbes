use super::AuditEventInput;
use serde_json::json;
use uuid::Uuid;

#[test]
fn audit_input_exposes_action_and_target_keys() {
    let input = AuditEventInput {
        tenant_id: Uuid::new_v4(),
        workspace_id: Some(Uuid::new_v4()),
        actor_principal_id: Some(Uuid::new_v4()),
        action: "identity.user.created",
        target_type: "user",
        target_id: Some(Uuid::new_v4()),
        ip: Some("203.0.113.10"),
        user_agent: Some("nvbes-test/1.0"),
        metadata: json!({ "source": "unit-test" }),
    };

    assert_eq!(input.action_key(), "identity.user.created");
    assert_eq!(input.target_key(), "user");
    assert!(input.is_workspace_scoped());
    assert!(input.has_actor());
    assert!(input.has_network_context());
}

#[test]
fn audit_input_can_be_tenant_scoped_without_workspace() {
    let input = AuditEventInput {
        tenant_id: Uuid::new_v4(),
        workspace_id: None,
        actor_principal_id: None,
        action: "tenant.policy.updated",
        target_type: "tenant",
        target_id: None,
        ip: None,
        user_agent: None,
        metadata: json!({}),
    };

    assert!(!input.is_workspace_scoped());
    assert!(!input.has_actor());
    assert!(!input.has_network_context());
}

#[test]
fn audit_input_partial_network_context_counts() {
    let with_ip = AuditEventInput {
        tenant_id: Uuid::new_v4(),
        workspace_id: None,
        actor_principal_id: None,
        action: "session.created",
        target_type: "session",
        target_id: Some(Uuid::new_v4()),
        ip: Some("198.51.100.7"),
        user_agent: None,
        metadata: json!({}),
    };
    assert!(with_ip.has_network_context());

    let with_ua = AuditEventInput {
        ip: None,
        user_agent: Some("curl/8.0"),
        ..with_ip.clone()
    };
    assert!(with_ua.has_network_context());
}
