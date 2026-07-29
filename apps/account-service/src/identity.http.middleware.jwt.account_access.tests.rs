use axum::http::StatusCode;
use uuid::Uuid;

use super::*;
use crate::http::middleware::jwt::OAuthBearerCredential;

#[test]
fn browser_session_is_first_party_without_oauth_scope() {
    let auth = auth_context(CredentialSource::BrowserSession, "");

    assert!(
        validate_account_access(
            &auth,
            AccountAccess::OAuthScope(EMAIL_WRITE_SCOPE),
            "nvbes-account-service"
        )
        .expect("browser session should be allowed")
        .is_none()
    );
}

#[test]
fn minimal_oauth_token_cannot_write_account_email() {
    let auth = oauth_auth_context(
        Some("customer-app"),
        "nvbes-account-service",
        "openid profile email",
    );

    let error = validate_account_access(
        &auth,
        AccountAccess::OAuthScope(EMAIL_WRITE_SCOPE),
        "nvbes-account-service",
    )
    .expect_err("minimal OAuth token must be denied");

    assert_eq!(error.status, StatusCode::FORBIDDEN);
    assert_eq!(error.code, "insufficient_scope");
}

#[test]
fn oauth_token_requires_account_audience() {
    let auth = oauth_auth_context(
        Some("customer-app"),
        "nvbes-cloud-service",
        EMAIL_WRITE_SCOPE,
    );

    let error = validate_account_access(
        &auth,
        AccountAccess::OAuthScope(EMAIL_WRITE_SCOPE),
        "nvbes-account-service",
    )
    .expect_err("foreign audience must be denied");

    assert_eq!(error.code, "account_token_audience_invalid");
}

#[test]
fn oauth_token_requires_client_id() {
    let auth = oauth_auth_context(None, "nvbes-account-service", EMAIL_WRITE_SCOPE);

    let error = validate_account_access(
        &auth,
        AccountAccess::OAuthScope(EMAIL_WRITE_SCOPE),
        "nvbes-account-service",
    )
    .expect_err("anonymous OAuth token must be denied");

    assert_eq!(error.code, "account_oauth_client_required");
}

#[test]
fn browser_only_operation_rejects_oauth_token() {
    let auth = oauth_auth_context(
        Some("customer-app"),
        "nvbes-account-service",
        SESSION_WRITE_SCOPE,
    );

    let error = validate_account_access(
        &auth,
        AccountAccess::BrowserSession,
        "nvbes-account-service",
    )
    .expect_err("OAuth token must not operate the browser session");

    assert_eq!(error.code, "browser_session_required");
}

#[test]
fn scoped_oauth_token_produces_current_policy_check() {
    let auth = oauth_auth_context(
        Some("customer-app"),
        "nvbes-account-service",
        &format!("openid {EMAIL_WRITE_SCOPE}"),
    );

    let check = validate_account_access(
        &auth,
        AccountAccess::OAuthScope(EMAIL_WRITE_SCOPE),
        "nvbes-account-service",
    )
    .expect("scoped OAuth token should pass claim validation")
    .expect("OAuth token should require a current policy check");

    assert_eq!(check.client_id, "customer-app");
    assert_eq!(check.required_scope, EMAIL_WRITE_SCOPE);
}

fn oauth_auth_context(client_id: Option<&str>, audience: &str, scope: &str) -> AuthContext {
    auth_context(
        CredentialSource::OAuthBearer(OAuthBearerCredential {
            client_id: client_id.map(str::to_string),
            audience: audience.to_string(),
            tenant_id: Some(Uuid::nil()),
            organization_id: None,
            workspace_id: None,
            workspace_region: None,
            acr: Some("aal1".to_string()),
            amr: vec!["pwd".to_string()],
            auth_time: None,
            cnf_jkt: None,
            cnf_x5t_s256: None,
        }),
        scope,
    )
}

fn auth_context(credential_source: CredentialSource, scope: &str) -> AuthContext {
    AuthContext {
        user_id: Uuid::nil(),
        user_email: "user@example.com".to_string(),
        display_name: "User".to_string(),
        email_verified_at: None,
        mfa_enabled: false,
        tenant_id: Some(Uuid::nil()),
        organization_id: None,
        workspace_id: None,
        workspace_region: None,
        token_type: "access".to_string(),
        scope: scope.to_string(),
        jti: Uuid::nil().to_string(),
        session_id: Uuid::nil(),
        acr: Some("aal1".to_string()),
        amr: vec!["pwd".to_string()],
        auth_time: None,
        client_id: None,
        cnf_jkt: None,
        cnf_x5t_s256: None,
        credential_source,
    }
}
