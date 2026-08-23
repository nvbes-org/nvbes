use uuid::Uuid;

use crate::billing_platform_center_routing_rule_simulation::{
    empty_to_none, matched_rule_from_grpc,
};
use crate::grpc_pb::nvbes::billing::v1::AdminBillingRoutingMatchedRule;

#[test]
fn empty_to_none_handles_empty_and_whitespace() {
    assert_eq!(empty_to_none("".to_string()), None);
    assert_eq!(empty_to_none("   ".to_string()), None);
    assert_eq!(empty_to_none("FR".to_string()), Some("FR".to_string()));
}

#[test]
fn matched_rule_from_grpc_parses_valid_rule() {
    let rule_id = Uuid::new_v4();
    let grpc_rule = AdminBillingRoutingMatchedRule {
        id: rule_id.to_string(),
        priority: 10,
        provider: "stripe".to_string(),
        fallback_enabled: true,
    };

    let result = matched_rule_from_grpc(grpc_rule).expect("valid rule should parse");
    assert_eq!(result.id, rule_id);
    assert_eq!(result.priority, 10);
    assert_eq!(result.provider, "stripe");
    assert!(result.fallback_enabled);
}

#[test]
fn matched_rule_from_grpc_rejects_invalid_uuid() {
    let grpc_rule = AdminBillingRoutingMatchedRule {
        id: "invalid-uuid".to_string(),
        priority: 10,
        provider: "stripe".to_string(),
        fallback_enabled: false,
    };

    assert!(matched_rule_from_grpc(grpc_rule).is_err());
}
