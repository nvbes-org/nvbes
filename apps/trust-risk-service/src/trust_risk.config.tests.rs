use super::{ConfigError, parse_operators, parse_producers};

const TOKEN: &str = "producer-token-with-at-least-32-characters";

#[test]
fn parses_explicit_producer_permissions() {
    let value = format!(
        r#"[{{"producer":"identity-service","token":"{TOKEN}","signal_prefixes":["identity.","automation."],"can_assess":true}}]"#
    );
    let policies = parse_producers(&value).unwrap();
    let policy = &policies["identity-service"];
    assert!(policy.permits_signal("identity.login"));
    assert!(!policy.permits_signal("payment.checkout"));
    assert!(policy.can_assess);
    assert!(!policy.can_label);
}

#[test]
fn rejects_duplicate_policies_and_weak_tokens() {
    let duplicate = format!(
        r#"[{{"producer":"identity-service","token":"{TOKEN}","signal_prefixes":[]}},{{"producer":"identity-service","token":"{TOKEN}","signal_prefixes":[]}}]"#
    );
    assert_eq!(
        parse_producers(&duplicate).unwrap_err(),
        ConfigError::DuplicatePolicy
    );
    assert_eq!(
        parse_operators(r#"[{"actor":"ada","token":"short","permissions":[]}]"#).unwrap_err(),
        ConfigError::WeakToken
    );
}

#[test]
fn operator_permissions_are_explicit() {
    let value = format!(
        r#"[{{"actor":"operator:ada","token":"{TOKEN}","permissions":["evaluation:read"]}}]"#
    );
    let policies = parse_operators(&value).unwrap();
    assert!(policies["operator:ada"].permits("evaluation:read"));
    assert!(!policies["operator:ada"].permits("rules:write"));
}
