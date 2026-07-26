use axum::http::HeaderMap;
use chrono::{DateTime, Utc};

use crate::domains::auth::ua_client_hints::UserAgentClientHints;
use crate::http::request;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRequestProfile {
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub accept_language: Option<String>,
    pub accept: Option<String>,
    pub accept_encoding: Option<String>,
    pub sec_fetch_site: Option<String>,
    pub sec_fetch_mode: Option<String>,
    pub sec_fetch_dest: Option<String>,
    pub ua_client_hints: UserAgentClientHints,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CookieTheftDecision {
    Allow,
    StepUp,
    Reauthenticate,
}

#[derive(Debug, Clone)]
pub struct CookieTheftAssessment {
    pub score: f64,
    pub decision: CookieTheftDecision,
    pub factors: Vec<&'static str>,
    pub assessed_at: DateTime<Utc>,
}

impl SessionRequestProfile {
    pub fn from_headers(headers: &HeaderMap) -> Self {
        Self {
            ip: request::client_ip(headers),
            user_agent: request::user_agent(headers),
            accept_language: header_value(headers, "Accept-Language"),
            accept: header_value(headers, "Accept"),
            accept_encoding: header_value(headers, "Accept-Encoding"),
            sec_fetch_site: header_value(headers, "Sec-Fetch-Site"),
            sec_fetch_mode: header_value(headers, "Sec-Fetch-Mode"),
            sec_fetch_dest: header_value(headers, "Sec-Fetch-Dest"),
            ua_client_hints: UserAgentClientHints::from_headers(headers),
        }
    }
}

pub fn apply_profile(
    session: &mut nvbes_redis::session::CachedSession,
    profile: &SessionRequestProfile,
) {
    session.ip = profile.ip.clone();
    session.user_agent = profile.user_agent.clone();
    session.accept_language = profile.accept_language.clone();
    session.accept = profile.accept.clone();
    session.accept_encoding = profile.accept_encoding.clone();
    session.sec_fetch_site = profile.sec_fetch_site.clone();
    session.sec_fetch_mode = profile.sec_fetch_mode.clone();
    session.sec_fetch_dest = profile.sec_fetch_dest.clone();
    profile.ua_client_hints.apply_to_session(session);
}

pub fn assess(
    session: &nvbes_redis::session::CachedSession,
    profile: &SessionRequestProfile,
) -> CookieTheftAssessment {
    let mut score = 0.0;
    let mut factors = Vec::new();

    add_if(
        &mut score,
        &mut factors,
        ip_network_marker(session.ip.as_deref()) != ip_network_marker(profile.ip.as_deref()),
        40.0,
        "ip_network_changed",
    );
    add_if(
        &mut score,
        &mut factors,
        user_agent_family(session.user_agent.as_deref())
            != user_agent_family(profile.user_agent.as_deref()),
        35.0,
        "user_agent_family_changed",
    );
    add_if(
        &mut score,
        &mut factors,
        primary_language(session.accept_language.as_deref())
            != primary_language(profile.accept_language.as_deref()),
        15.0,
        "accept_language_changed",
    );
    add_if(
        &mut score,
        &mut factors,
        normalized(session.accept.as_deref()) != normalized(profile.accept.as_deref()),
        8.0,
        "accept_header_changed",
    );
    add_if(
        &mut score,
        &mut factors,
        normalized(session.accept_encoding.as_deref())
            != normalized(profile.accept_encoding.as_deref()),
        6.0,
        "accept_encoding_changed",
    );
    let previous_hints = UserAgentClientHints::from_session(session);
    add_if(
        &mut score,
        &mut factors,
        changed(
            previous_hints.platform.as_deref(),
            profile.ua_client_hints.platform.as_deref(),
        ),
        20.0,
        "client_platform_changed",
    );
    add_if(
        &mut score,
        &mut factors,
        changed(
            previous_hints.mobile.as_deref(),
            profile.ua_client_hints.mobile.as_deref(),
        ),
        10.0,
        "client_mobile_hint_changed",
    );
    add_if(
        &mut score,
        &mut factors,
        changed_group(
            [
                previous_hints.architecture.as_deref(),
                previous_hints.bitness.as_deref(),
                previous_hints.wow64.as_deref(),
            ],
            [
                profile.ua_client_hints.architecture.as_deref(),
                profile.ua_client_hints.bitness.as_deref(),
                profile.ua_client_hints.wow64.as_deref(),
            ],
        ),
        15.0,
        "client_architecture_changed",
    );
    add_if(
        &mut score,
        &mut factors,
        changed(
            previous_hints.form_factors.as_deref(),
            profile.ua_client_hints.form_factors.as_deref(),
        ),
        10.0,
        "client_form_factor_changed",
    );
    add_if(
        &mut score,
        &mut factors,
        changed(
            previous_hints.model.as_deref(),
            profile.ua_client_hints.model.as_deref(),
        ),
        15.0,
        "client_model_changed",
    );
    add_if(
        &mut score,
        &mut factors,
        suspicious_fetch_site(profile.sec_fetch_site.as_deref()),
        20.0,
        "cross_site_session_use",
    );

    CookieTheftAssessment {
        score,
        decision: decision_for_score(score),
        factors,
        assessed_at: Utc::now(),
    }
}

pub fn has_detection_profile(session: &nvbes_redis::session::CachedSession) -> bool {
    session.ip.is_some()
        || session.user_agent.is_some()
        || session.accept_language.is_some()
        || session.accept.is_some()
        || session.sec_ch_ua_platform.is_some()
}

fn header_value(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.chars().take(256).collect())
}

fn add_if(
    score: &mut f64,
    factors: &mut Vec<&'static str>,
    condition: bool,
    weight: f64,
    factor: &'static str,
) {
    if condition {
        *score += weight;
        factors.push(factor);
    }
}

fn decision_for_score(score: f64) -> CookieTheftDecision {
    if score >= 70.0 {
        CookieTheftDecision::Reauthenticate
    } else if score >= 40.0 {
        CookieTheftDecision::StepUp
    } else {
        CookieTheftDecision::Allow
    }
}

fn normalized(value: Option<&str>) -> Option<String> {
    value.map(|value| value.trim().to_ascii_lowercase())
}

fn changed(previous: Option<&str>, current: Option<&str>) -> bool {
    matches!((normalized(previous), normalized(current)), (Some(previous), Some(current)) if previous != current)
}

fn changed_group<const N: usize>(previous: [Option<&str>; N], current: [Option<&str>; N]) -> bool {
    previous
        .into_iter()
        .zip(current)
        .any(|(previous, current)| changed(previous, current))
}

fn primary_language(value: Option<&str>) -> Option<String> {
    let language = value?.split(',').next()?.trim().split('-').next()?;
    Some(language.to_ascii_lowercase())
}

fn user_agent_family(value: Option<&str>) -> Option<String> {
    let value = value?.to_ascii_lowercase();
    let browser = if value.contains("firefox/") {
        "firefox"
    } else if value.contains("edg/") {
        "edge"
    } else if value.contains("chrome/") || value.contains("chromium/") {
        "chromium"
    } else if value.contains("safari/") {
        "safari"
    } else {
        "other"
    };
    let os = if value.contains("windows") {
        "windows"
    } else if value.contains("android") {
        "android"
    } else if value.contains("iphone") || value.contains("ipad") {
        "ios"
    } else if value.contains("mac os") || value.contains("macintosh") {
        "macos"
    } else if value.contains("linux") {
        "linux"
    } else {
        "unknown"
    };
    Some(format!("{browser}:{os}"))
}

fn ip_network_marker(value: Option<&str>) -> Option<String> {
    let value = value?;
    if value.contains(':') {
        return Some(value.split(':').take(4).collect::<Vec<_>>().join(":"));
    }
    Some(value.split('.').take(2).collect::<Vec<_>>().join("."))
}

fn suspicious_fetch_site(value: Option<&str>) -> bool {
    matches!(normalized(value).as_deref(), Some("cross-site"))
}

#[cfg(test)]
#[path = "identity.domains.auth.sessions.cookie_theft.tests.rs"]
mod tests;
