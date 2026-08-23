use std::time::Duration;

use super::*;
use crate::domains::auth::identity::cache::{
    get_cached_introspection, hash_token, set_cached_introspection,
};

#[test]
fn test_introspection_cache_operations() {
    let token = "test_token_123";
    let token_hash = hash_token(token);

    let response = IdentityIntrospectionResponse {
        active: true,
        scope: Some("read write".to_string()),
        client_id: Some("client1".to_string()),
        principal_type: Some("user".to_string()),
        token_type: Some("access_token".to_string()),
        audience: Some("nvbes-cloud-service".to_string()),
        sub: Some("user1".to_string()),
        role: Some("admin".to_string()),
        tenant_id: None,
        organization_id: None,
        workspace_id: None,
        username: None,
        email: None,
        email_verified: None,
        name: None,
        acr: None,
        amr: vec![],
        auth_time: None,
        jti: None,
        sid: Some("session123".to_string()),
        exp: None,
        iat: None,
        nbf: None,
        act: None,
        actor_principal_type: None,
        actor_role: None,
        actor_workspace_id: None,
        actor_organization_id: None,
        actor_tenant_id: None,
        network_valid: None,
    };

    assert!(get_cached_introspection(&token_hash).is_none());
    set_cached_introspection(token_hash.clone(), response.clone(), Duration::from_secs(5));
    let cached = get_cached_introspection(&token_hash).expect("cache hit");
    assert_eq!(cached.sid, Some("session123".to_string()));

    invalidate_cached_session("session123");
    assert!(get_cached_introspection(&token_hash).is_none());

    set_cached_introspection(token_hash.clone(), response, Duration::from_millis(1));
    std::thread::sleep(Duration::from_millis(2));
    assert!(get_cached_introspection(&token_hash).is_none());
}
