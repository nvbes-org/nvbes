use super::hosted_keys::{hosted_authorization_state_key, hosted_authorization_state_ttl_seconds};

#[test]
fn hosted_authorization_state_key_is_namespaced() {
    let key = hosted_authorization_state_key("state_123");
    assert_eq!(key, "nvbes:identity:oauth:hosted-login:state_123");
}

#[test]
fn hosted_authorization_state_ttl_is_short_lived() {
    assert_eq!(hosted_authorization_state_ttl_seconds(), 300);
}
