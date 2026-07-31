use chrono::{TimeZone, Utc};
use uuid::Uuid;

use super::{AuthContext, CredentialSource, OAuthBearerCredential};

#[test]
fn browser_authentication_keeps_session_context() {
    let tenant_id = Uuid::new_v4();
    let workspace_id = Uuid::new_v4();
    let auth = session_auth_context(tenant_id, workspace_id);

    let context = AuthContext::from_authenticated_session(auth, CredentialSource::BrowserSession);

    assert_eq!(context.tenant_id, Some(tenant_id));
    assert_eq!(context.workspace_id, Some(workspace_id));
    assert_eq!(context.client_id.as_deref(), Some("account-web"));
    assert_eq!(context.acr.as_deref(), Some("aal2"));
}

#[test]
fn oauth_authentication_uses_signed_token_context() {
    let session_tenant_id = Uuid::new_v4();
    let session_workspace_id = Uuid::new_v4();
    let token_tenant_id = Uuid::new_v4();
    let token_workspace_id = Uuid::new_v4();
    let auth = session_auth_context(session_tenant_id, session_workspace_id);
    let credential = OAuthBearerCredential {
        client_id: Some("customer-app".to_string()),
        audience: "nvbes-account-service".to_string(),
        tenant_id: Some(token_tenant_id),
        organization_id: None,
        workspace_id: Some(token_workspace_id),
        workspace_region: Some("us".to_string()),
        acr: Some("aal1".to_string()),
        amr: vec!["pwd".to_string()],
        auth_time: Some(1_700_000_000),
        cnf_jkt: None,
        cnf_x5t_s256: None,
    };

    let context = AuthContext::from_authenticated_session(
        auth,
        CredentialSource::OAuthBearer(Box::new(credential)),
    );

    assert_eq!(context.tenant_id, Some(token_tenant_id));
    assert_eq!(context.workspace_id, Some(token_workspace_id));
    assert_eq!(context.workspace_region.as_deref(), Some("us"));
    assert_eq!(context.client_id.as_deref(), Some("customer-app"));
    assert_eq!(context.acr.as_deref(), Some("aal1"));
    assert_eq!(context.amr, ["pwd"]);
    assert_eq!(context.auth_time, Some(1_700_000_000));
}

fn session_auth_context(
    tenant_id: Uuid,
    workspace_id: Uuid,
) -> crate::domains::auth::types::AuthContext {
    crate::domains::auth::types::AuthContext {
        user_id: Uuid::new_v4(),
        user_email: "user@example.com".to_string(),
        display_name: "User".to_string(),
        email_verified_at: None,
        mfa_enabled: true,
        tenant_id: Some(tenant_id),
        organization_id: None,
        session_id: Uuid::new_v4(),
        workspace_id: Some(workspace_id),
        workspace_region: Some("eu".to_string()),
        scope: "openid profile email".to_string(),
        acr: Some("aal2".to_string()),
        amr: vec!["pwd".to_string(), "totp".to_string()],
        auth_time: Some(Utc.timestamp_opt(1_800_000_000, 0).unwrap()),
        client_id: Some("account-web".to_string()),
        cnf_jkt: None,
    }
}
