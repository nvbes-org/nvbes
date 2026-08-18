use crate::app::AppState;
use crate::http::middleware::jwt::account_access::{
    self, AccountAccess, EMAIL_READ_SCOPE, EMAIL_WRITE_SCOPE, SECURITY_WRITE_SCOPE,
    SESSION_READ_SCOPE, SESSION_WRITE_SCOPE,
};
use axum::{
    Router,
    routing::{delete, get, post},
};

use super::devices::{revoke_device, trust_device};
use super::{
    confirm_high_risk_session, forget_account_cookie, get_accounts, list_sessions, logout,
    me_email_delete, me_email_promote, me_email_resend_verification, me_emails_get, me_emails_post,
    revoke_all_other_sessions, revoke_all_sessions, revoke_session, step_up,
    step_up::request_email_step_up, switch_workspace::switch_workspace,
};

pub(super) fn router(state: &AppState) -> Router<AppState> {
    Router::new()
        .route("/accounts", axum::routing::get(get_accounts))
        .route(
            "/accounts/{authuser}",
            axum::routing::delete(forget_account_cookie),
        )
        .route(
            "/logout",
            account_access::protected_method(state, AccountAccess::BrowserSession, post(logout)),
        )
        .route(
            "/sessions",
            account_access::protected_method(
                state,
                AccountAccess::OAuthScope(SESSION_READ_SCOPE),
                get(list_sessions),
            ),
        )
        .route(
            "/sessions/{sessionId}",
            account_access::protected_method(
                state,
                AccountAccess::OAuthScope(SESSION_WRITE_SCOPE),
                delete(revoke_session),
            ),
        )
        .route(
            "/sessions/{sessionId}/confirm",
            account_access::protected_method(
                state,
                AccountAccess::OAuthScope(SESSION_WRITE_SCOPE),
                post(confirm_high_risk_session),
            ),
        )
        .route(
            "/sessions/revoke-others",
            account_access::protected_method(
                state,
                AccountAccess::OAuthScope(SESSION_WRITE_SCOPE),
                post(revoke_all_other_sessions),
            ),
        )
        .route(
            "/sessions/revoke-all",
            account_access::protected_method(
                state,
                AccountAccess::OAuthScope(SESSION_WRITE_SCOPE),
                post(revoke_all_sessions),
            ),
        )
        .route(
            "/step-up/email/request",
            account_access::protected_method(
                state,
                AccountAccess::OAuthScope(SECURITY_WRITE_SCOPE),
                post(request_email_step_up),
            ),
        )
        .route(
            "/devices/{deviceId}/trust",
            account_access::protected_method(
                state,
                AccountAccess::OAuthScope(SECURITY_WRITE_SCOPE),
                post(trust_device),
            ),
        )
        .route(
            "/devices/{deviceId}",
            account_access::protected_method(
                state,
                AccountAccess::OAuthScope(SECURITY_WRITE_SCOPE),
                delete(revoke_device),
            ),
        )
        .route(
            "/me/emails",
            account_access::protected_method(
                state,
                AccountAccess::OAuthScope(EMAIL_READ_SCOPE),
                get(me_emails_get),
            ),
        )
        .route(
            "/me/emails",
            account_access::protected_method(
                state,
                AccountAccess::OAuthScope(EMAIL_WRITE_SCOPE),
                post(me_emails_post),
            ),
        )
        .route(
            "/me/emails/{emailId}/promote",
            account_access::protected_method(
                state,
                AccountAccess::OAuthScope(EMAIL_WRITE_SCOPE),
                post(me_email_promote),
            ),
        )
        .route(
            "/me/emails/{emailId}/resend-verification",
            account_access::protected_method(
                state,
                AccountAccess::OAuthScope(EMAIL_WRITE_SCOPE),
                post(me_email_resend_verification),
            ),
        )
        .route(
            "/me/emails/{emailId}",
            account_access::protected_method(
                state,
                AccountAccess::OAuthScope(EMAIL_WRITE_SCOPE),
                delete(me_email_delete),
            ),
        )
        .route(
            "/step-up",
            account_access::protected_method(
                state,
                AccountAccess::OAuthScope(SECURITY_WRITE_SCOPE),
                post(step_up),
            ),
        )
        .route(
            "/workspaces/{workspaceId}/switch",
            account_access::protected_method(
                state,
                AccountAccess::OAuthScope(SESSION_WRITE_SCOPE),
                post(switch_workspace),
            ),
        )
}
