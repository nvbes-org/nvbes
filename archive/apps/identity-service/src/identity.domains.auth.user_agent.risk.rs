use crate::domains::auth::bot_signals::BotSignals;

use super::{UserAgentInfo, parse};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct UserAgentRiskAssessment {
    pub score: f64,
    pub factors: Vec<&'static str>,
    pub client: Option<UserAgentInfo>,
}

pub fn assess(
    user_agent: Option<&str>,
    browser_brands: Option<&str>,
    signals: Option<&BotSignals>,
) -> UserAgentRiskAssessment {
    let Some(user_agent) = user_agent.map(str::trim).filter(|value| !value.is_empty()) else {
        return UserAgentRiskAssessment::default();
    };
    let client = parse(Some(user_agent), browser_brands);
    let mut assessment = UserAgentRiskAssessment {
        client,
        ..UserAgentRiskAssessment::default()
    };
    let lower = user_agent.to_ascii_lowercase();

    if [
        "headlesschrome",
        "phantomjs",
        "selenium",
        "playwright",
        "puppeteer",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
    {
        assessment.add(0.95, "ua_automation_signature");
    }

    let looks_like_browser =
        lower.contains("mozilla/") || lower.contains("chrome/") || lower.contains("safari/");
    if looks_like_browser
        && assessment
            .client
            .as_ref()
            .is_some_and(|client| client.browser.is_none())
    {
        assessment.add(0.08, "ua_browser_unrecognized");
    }
    if looks_like_browser
        && assessment
            .client
            .as_ref()
            .is_some_and(|client| client.os.is_none())
    {
        assessment.add(0.05, "ua_os_unrecognized");
    }

    if let (Some(client), Some(signals)) = (assessment.client.clone(), signals) {
        let touch_expected = matches!(
            client.device_type.as_str(),
            "mobile" | "tablet" | "wearable"
        );
        if touch_expected && signals.nav_touch_points == Some(0) {
            assessment.add(0.15, "ua_touch_capability_mismatch");
        }
        if let Some(expected_platform) = normalized_os_platform(client.os.as_deref())
            && let Some(reported_platform) = signals.ua_platform.as_deref().map(normalized_platform)
            && expected_platform != reported_platform
        {
            assessment.add(0.15, "ua_js_platform_mismatch");
        }
        if let Some(reported_mobile) = signals.ua_mobile {
            let expected_mobile = matches!(client.device_type.as_str(), "mobile" | "tablet");
            if expected_mobile != reported_mobile {
                assessment.add(0.12, "ua_js_mobile_mismatch");
            }
        }
        assess_screen_form_factor(&mut assessment, signals);
    }

    assessment.score = assessment.score.min(1.0);
    assessment
}

impl UserAgentRiskAssessment {
    fn add(&mut self, score: f64, factor: &'static str) {
        self.score += score;
        self.factors.push(factor);
    }
}

fn assess_screen_form_factor(assessment: &mut UserAgentRiskAssessment, signals: &BotSignals) {
    let Some(client) = assessment.client.as_ref() else {
        return;
    };
    if !matches!(client.device_type.as_str(), "mobile" | "tablet") {
        return;
    }
    let (Some(width), Some(height)) = (signals.screen_width, signals.screen_height) else {
        return;
    };
    let short_side = width.min(height);
    let screen_type = if short_side >= 600 {
        "tablet"
    } else {
        "mobile"
    };
    if screen_type != client.device_type {
        assessment.add(0.10, "ua_screen_form_factor_mismatch");
    }
}

fn normalized_os_platform(os: Option<&str>) -> Option<&'static str> {
    match os? {
        "Android" => Some("android"),
        "Chrome OS" => Some("chrome os"),
        "iOS" | "watchOS" => Some("ios"),
        "Linux" => Some("linux"),
        "Mac OS X" => Some("macos"),
        "Windows" | "Windows Mobile" | "Windows Phone" => Some("windows"),
        _ => None,
    }
}

fn normalized_platform(value: &str) -> &'static str {
    match value.trim().to_ascii_lowercase().as_str() {
        "android" => "android",
        "chrome os" | "chromium os" => "chrome os",
        "ios" => "ios",
        "linux" => "linux",
        "macos" | "mac os" => "macos",
        "windows" => "windows",
        _ => "unknown",
    }
}
