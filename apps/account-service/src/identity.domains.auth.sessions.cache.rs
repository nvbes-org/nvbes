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
    browser_session_token_hash: String,
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
        browser_session_token_hash: Some(browser_session_token_hash),
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
        idle_timeout_seconds: None,
        idle_expires_at: None,
        expires_at,
        revoked_at: None,
        ip,
        geo_country_code: None,
        user_agent,
        accept_language: None,
        accept: None,
        accept_encoding: None,
        sec_fetch_site: None,
        sec_fetch_mode: None,
        sec_fetch_dest: None,
        sec_ch_ua: None,
        sec_ch_ua_arch: None,
        sec_ch_ua_bitness: None,
        sec_ch_ua_full_version: None,
        sec_ch_ua_full_version_list: None,
        sec_ch_ua_model: None,
        sec_ch_ua_wow64: None,
        sec_ch_ua_form_factors: None,
        sec_ch_ua_platform: None,
        sec_ch_ua_platform_version: None,
        sec_ch_ua_mobile: None,
        cookie_theft_risk_score: None,
        cookie_theft_detected_at: None,
        account_device_id: None,
        device_trust_level: None,
        device_trust_score: None,
        risk_score: None,
        risk_decision: None,
        risk_confirmed_at: None,
        risk_confirmed_score: None,
        activity_window_started_at: Some(created_at),
        activity_request_count: 0,
        last_activity_risk_event_at: None,
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
        geo_country_code: session.geo_country_code.clone(),
        user_agent: session.user_agent.clone(),
        client: crate::domains::auth::user_agent::parse(
            session.user_agent.as_deref(),
            session.sec_ch_ua.as_deref(),
        ),
        device_id: session
            .account_device_id
            .as_deref()
            .and_then(|value| Uuid::parse_str(value).ok()),
        device_trust_level: session.device_trust_level.clone(),
        device_trust_score: session.device_trust_score,
        risk_score: session.risk_score,
        risk_decision: session.risk_decision.clone(),
        risk_confirmed_at: session.risk_confirmed_at,
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

pub fn configure_idle_timeout(
    session: &mut CachedSession,
    idle_timeout_minutes: i64,
    now: DateTime<Utc>,
) {
    if idle_timeout_minutes <= 0 {
        session.idle_timeout_seconds = None;
        session.idle_expires_at = None;
        return;
    }

    let idle_timeout = ChronoDuration::minutes(idle_timeout_minutes);
    session.idle_timeout_seconds = Some(idle_timeout.num_seconds());
    session.idle_expires_at = Some((now + idle_timeout).min(session.expires_at));
}

pub fn refresh_last_seen(session: &mut CachedSession) {
    refresh_last_seen_at(session, Utc::now());
}

fn refresh_last_seen_at(session: &mut CachedSession, now: DateTime<Utc>) {
    session.last_seen_at = now;
    if let Some(idle_timeout_seconds) = session.idle_timeout_seconds {
        session.idle_expires_at =
            Some((now + ChronoDuration::seconds(idle_timeout_seconds)).min(session.expires_at));
    }
}

pub fn current_session_ttl(session: &CachedSession) -> u64 {
    session_ttl_seconds(
        session
            .idle_expires_at
            .unwrap_or(session.expires_at)
            .min(session.expires_at),
    )
}

pub fn is_expired(session: &CachedSession, now: DateTime<Utc>) -> bool {
    session.revoked_at.is_some()
        || session.expires_at <= now
        || session
            .idle_expires_at
            .is_some_and(|expires_at| expires_at <= now)
}

pub fn expires_at_from_ttl(ttl_hours: i64) -> DateTime<Utc> {
    Utc::now() + ChronoDuration::hours(ttl_hours)
}

#[cfg(test)]
mod idle_timeout_tests {
    use super::*;

    fn session(now: DateTime<Utc>, absolute_ttl: ChronoDuration) -> CachedSession {
        cached_session_from_login(
            Uuid::new_v4(),
            Uuid::new_v4(),
            None,
            None,
            None,
            None,
            None,
            "token-hash".to_string(),
            Some("aal1".to_string()),
            vec!["pwd".to_string()],
            now,
            now,
            None,
            None,
            now + absolute_ttl,
        )
    }

    #[test]
    fn idle_deadline_slides_with_activity() {
        let now = Utc::now();
        let mut session = session(now, ChronoDuration::days(30));
        configure_idle_timeout(&mut session, 30, now);

        refresh_last_seen_at(&mut session, now + ChronoDuration::minutes(10));

        assert_eq!(session.last_seen_at, now + ChronoDuration::minutes(10));
        assert_eq!(
            session.idle_expires_at,
            Some(now + ChronoDuration::minutes(40))
        );
    }

    #[test]
    fn idle_deadline_never_exceeds_absolute_expiration() {
        let now = Utc::now();
        let mut session = session(now, ChronoDuration::minutes(20));

        configure_idle_timeout(&mut session, 30, now);
        refresh_last_seen_at(&mut session, now + ChronoDuration::minutes(10));

        assert_eq!(session.idle_expires_at, Some(session.expires_at));
    }

    #[test]
    fn idle_expiration_invalidates_session_before_absolute_deadline() {
        let now = Utc::now();
        let mut session = session(now, ChronoDuration::days(30));
        configure_idle_timeout(&mut session, 30, now);

        assert!(!is_expired(&session, now + ChronoDuration::minutes(29)));
        assert!(is_expired(&session, now + ChronoDuration::minutes(30)));
    }

    #[test]
    fn non_positive_idle_timeout_disables_idle_expiration() {
        let now = Utc::now();
        let mut session = session(now, ChronoDuration::hours(1));

        configure_idle_timeout(&mut session, 0, now);

        assert_eq!(session.idle_timeout_seconds, None);
        assert_eq!(session.idle_expires_at, None);
    }

    #[test]
    fn session_view_includes_resolved_country() {
        let now = Utc::now();
        let mut session = session(now, ChronoDuration::hours(1));
        session.geo_country_code = Some("FR".to_string());

        let view = session_view_from_cached_session(&session, true);

        assert_eq!(view.geo_country_code.as_deref(), Some("FR"));
    }

    #[test]
    fn session_view_includes_normalized_client() {
        let now = Utc::now();
        let mut session = session(now, ChronoDuration::hours(1));
        session.user_agent = Some(
            "Mozilla/5.0 (iPhone; CPU iPhone OS 17_5 like Mac OS X) \
             Version/17.5 Mobile/15E148 Safari/604.1"
                .to_string(),
        );

        let view = session_view_from_cached_session(&session, true);
        let client = view.client.expect("client should be parsed");

        assert_eq!(client.browser.as_deref(), Some("Mobile Safari"));
        assert_eq!(client.os.as_deref(), Some("iOS"));
        assert_eq!(client.device.as_deref(), Some("iPhone"));
        assert_eq!(client.device_type, "mobile");
    }
}
