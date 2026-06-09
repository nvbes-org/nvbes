use chrono::{DateTime, Duration as ChronoDuration, Utc};
use uuid::Uuid;

use crate::domains::auth::db::UserRecord;
use crate::domains::auth::types::{AuthContext, SessionView};

pub use nvbes_redis::session::CachedSession;

pub fn session_ttl_seconds(expires_at: DateTime<Utc>) -> u64 {
    let ttl = (expires_at - Utc::now()).num_seconds().max(1);
    ttl as u64
}

#[expect(
    clippy::too_many_arguments,
    reason = "Cached session materialization keeps auth context fields explicit."
)]
pub fn cached_session_from_login(
    session_id: Uuid,
    principal_id: Uuid,
    tenant_id: Option<Uuid>,
    organization_id: Option<Uuid>,
    workspace_id: Option<Uuid>,
    workspace_region: Option<String>,
    client_id: Option<String>,
    token_hash: String,
    acr: Option<String>,
    amr: Vec<String>,
    auth_time: DateTime<Utc>,
    created_at: DateTime<Utc>,
    ip: Option<String>,
    user_agent: Option<String>,
    expires_at: DateTime<Utc>,
) -> CachedSession {
    CachedSession {
        session_id: session_id.to_string(),
        principal_id: principal_id.to_string(),
        token_hash,
        tenant_id: tenant_id.map(|value| value.to_string()),
        organization_id: organization_id.map(|value| value.to_string()),
        workspace_id: workspace_id.map(|value| value.to_string()),
        workspace_region,
        client_id,
        acr,
        amr,
        auth_time: Some(auth_time),
        step_up_verified_at: None,
        step_up_expires_at: None,
        created_at,
        last_seen_at: created_at,
        expires_at,
        revoked_at: None,
        ip,
        user_agent,
    }
}

pub fn session_view_from_cached_session(session: &CachedSession, current: bool) -> SessionView {
    SessionView {
        id: Uuid::parse_str(&session.session_id).expect("cached session id should be valid"),
        tenant_id: session
            .tenant_id
            .as_deref()
            .and_then(|value| Uuid::parse_str(value).ok()),
        organization_id: session
            .organization_id
            .as_deref()
            .and_then(|value| Uuid::parse_str(value).ok()),
        workspace_id: session
            .workspace_id
            .as_deref()
            .and_then(|value| Uuid::parse_str(value).ok()),
        workspace_region: session.workspace_region.clone(),
        created_at: session.created_at,
        last_seen_at: session.last_seen_at,
        expires_at: session.expires_at,
        revoked_at: session.revoked_at,
        ip: session.ip.clone(),
        user_agent: session.user_agent.clone(),
        current,
    }
}

pub fn auth_context_from_cached_session(
    session: &CachedSession,
    user: &UserRecord,
    mfa_enabled: bool,
    scope: String,
    cnf_jkt: Option<String>,
) -> AuthContext {
    AuthContext {
        user_id: user.principal_id,
        user_email: user.email.clone(),
        display_name: user.display_name.clone(),
        email_verified_at: user.email_verified_at,
        mfa_enabled,
        tenant_id: session
            .tenant_id
            .as_deref()
            .and_then(|value| Uuid::parse_str(value).ok()),
        organization_id: session
            .organization_id
            .as_deref()
            .and_then(|value| Uuid::parse_str(value).ok()),
        session_id: Uuid::parse_str(&session.session_id)
            .expect("cached session id should be valid"),
        workspace_id: session
            .workspace_id
            .as_deref()
            .and_then(|value| Uuid::parse_str(value).ok()),
        workspace_region: session.workspace_region.clone(),
        scope,
        acr: session.acr.clone(),
        amr: session.amr.clone(),
        auth_time: session.auth_time,
        client_id: session.client_id.clone(),
        cnf_jkt,
    }
}

pub fn apply_step_up(
    session: &mut CachedSession,
    acr: &str,
    amr: Vec<String>,
    auth_time: DateTime<Utc>,
    step_up_expires_at: DateTime<Utc>,
) {
    session.acr = Some(acr.to_string());
    session.amr = amr;
    session.auth_time = Some(auth_time);
    session.step_up_verified_at = Some(auth_time);
    session.step_up_expires_at = Some(step_up_expires_at);
}

pub fn apply_workspace_context(
    session: &mut CachedSession,
    tenant_id: Option<Uuid>,
    organization_id: Option<Uuid>,
    workspace_id: Option<Uuid>,
    workspace_region: Option<String>,
) {
    session.tenant_id = tenant_id.map(|value| value.to_string());
    session.organization_id = organization_id.map(|value| value.to_string());
    session.workspace_id = workspace_id.map(|value| value.to_string());
    session.workspace_region = workspace_region;
}

pub fn apply_token_rotation(session: &mut CachedSession, token_hash: String) {
    session.token_hash = token_hash;
}

pub fn refresh_last_seen(session: &mut CachedSession) {
    session.last_seen_at = Utc::now();
}

pub fn current_session_ttl(session: &CachedSession) -> u64 {
    session_ttl_seconds(session.expires_at)
}

pub fn expires_at_from_ttl(ttl_hours: i64) -> DateTime<Utc> {
    Utc::now() + ChronoDuration::hours(ttl_hours)
}
