use super::{CookieTheftDecision, SessionRequestProfile, apply_profile, assess};
use crate::domains::auth::sessions::cache::cached_session_from_login;
use crate::domains::auth::ua_client_hints::UserAgentClientHints;
use chrono::Utc;
use uuid::Uuid;

fn session() -> nvbes_redis::session::CachedSession {
    let now = Utc::now();
    let mut session = cached_session_from_login(
        Uuid::new_v4(),
        Uuid::new_v4(),
        None,
        None,
        None,
        None,
        None,
        "hash".to_string(),
        None,
        vec![],
        now,
        now,
        None,
        None,
        now + chrono::Duration::hours(1),
    );
    apply_profile(&mut session, &stable_profile());
    session
}

fn stable_profile() -> SessionRequestProfile {
    SessionRequestProfile {
        ip: Some("203.0.113.24".to_string()),
        user_agent: Some("Mozilla/5.0 Chrome/120.0 Safari/537.36".to_string()),
        accept_language: Some("en-US,en;q=0.9".to_string()),
        accept: Some("application/json".to_string()),
        accept_encoding: Some("gzip, br".to_string()),
        sec_fetch_site: Some("same-origin".to_string()),
        sec_fetch_mode: Some("cors".to_string()),
        sec_fetch_dest: Some("empty".to_string()),
        ua_client_hints: UserAgentClientHints {
            brands: Some("\"Chromium\";v=\"120\"".to_string()),
            architecture: Some("\"arm\"".to_string()),
            bitness: Some("\"64\"".to_string()),
            form_factors: Some("\"Desktop\"".to_string()),
            model: Some("\"Stable device\"".to_string()),
            mobile: Some("?0".to_string()),
            platform: Some("\"macOS\"".to_string()),
            ..Default::default()
        },
    }
}

#[test]
fn major_environment_change_requires_reauthentication() {
    let mut profile = stable_profile();
    profile.ip = Some("198.51.100.8".to_string());
    profile.user_agent = Some("Mozilla/5.0 Firefox/121.0".to_string());
    profile.accept_language = Some("fr-FR,fr;q=0.9".to_string());

    let assessment = assess(&session(), &profile);

    assert_eq!(assessment.decision, CookieTheftDecision::Reauthenticate);
    assert!(assessment.factors.contains(&"ip_network_changed"));
    assert!(assessment.factors.contains(&"user_agent_family_changed"));
}

#[test]
fn architecture_and_model_changes_raise_session_risk() {
    let mut profile = stable_profile();
    profile.ua_client_hints.architecture = Some("\"x86\"".to_string());
    profile.ua_client_hints.model = Some("\"Different device\"".to_string());

    let assessment = assess(&session(), &profile);

    assert_eq!(assessment.decision, CookieTheftDecision::Allow);
    assert_eq!(assessment.score, 30.0);
    assert!(assessment.factors.contains(&"client_architecture_changed"));
    assert!(assessment.factors.contains(&"client_model_changed"));
}

#[test]
fn browser_and_platform_updates_do_not_rotate_the_device_session() {
    let mut baseline = stable_profile();
    baseline.ua_client_hints.full_version = Some("\"126.0.1.2\"".to_string());
    baseline.ua_client_hints.full_version_list = Some("\"Chromium\";v=\"126.0.1.2\"".to_string());
    baseline.ua_client_hints.platform_version = Some("\"14.0.0\"".to_string());
    let mut session = session();
    apply_profile(&mut session, &baseline);

    let mut current = baseline;
    current.ua_client_hints.full_version = Some("\"127.0.0.1\"".to_string());
    current.ua_client_hints.full_version_list = Some("\"Chromium\";v=\"127.0.0.1\"".to_string());
    current.ua_client_hints.platform_version = Some("\"15.0.0\"".to_string());

    assert_eq!(assess(&session, &current).score, 0.0);
}

#[test]
fn browser_version_update_keeps_the_same_session_family() {
    let mut baseline = stable_profile();
    baseline.user_agent =
        Some("Mozilla/5.0 (Windows NT 10.0) Chrome/126.0 Safari/537.36".to_string());
    let mut session = session();
    apply_profile(&mut session, &baseline);

    let mut current = baseline;
    current.user_agent =
        Some("Mozilla/5.0 (Windows NT 10.0) Chrome/127.0 Safari/537.36".to_string());

    let assessment = assess(&session, &current);
    assert!(!assessment.factors.contains(&"user_agent_family_changed"));
}
