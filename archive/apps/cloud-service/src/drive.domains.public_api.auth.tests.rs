use uuid::Uuid;

use super::authorize::auth_context;
use super::scopes::scope_allows_required;
use crate::domains::auth::types::AuthPrincipalKind;
use crate::domains::public_api::types::PublicApiContext;

fn public_api_context(role: Option<String>, m2m_client_id: Option<String>) -> PublicApiContext {
    PublicApiContext {
        api_key_id: None,
        workspace_id: Uuid::new_v4(),
        created_by: None,
        created_by_principal_id: Uuid::new_v4(),
        tenant_id: Some(Uuid::new_v4()),
        organization_id: None,
        role,
        key_prefix: "m2m".to_string(),
        scopes: vec!["drive.share_links.write".to_string()],
        plan_code: "team".to_string(),
        request_id: "test-request".to_string(),
        m2m_client_id,
    }
}

#[test]
fn auth_context_preserves_m2m_workspace_role() {
    let context = public_api_context(Some("member".to_string()), Some("client".to_string()));
    let auth = auth_context(&context);

    assert_eq!(auth.role.as_deref(), Some("member"));
    assert_eq!(auth.principal_kind, AuthPrincipalKind::ServiceAccount);
}

#[test]
fn scope_allows_public_scope_alias_and_drive_scope() {
    assert!(scope_allows_required(
        &["drive.share_links.write".to_string()],
        "share_links:write"
    ));
    assert!(scope_allows_required(
        &["share_links:write".to_string()],
        "share_links:write"
    ));
    assert!(scope_allows_required(
        &["drive:share_link:write".to_string()],
        "share_links:write"
    ));
    assert!(!scope_allows_required(
        &["drive.files.read".to_string()],
        "share_links:write"
    ));
}
