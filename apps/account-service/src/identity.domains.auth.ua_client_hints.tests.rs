use super::*;
use crate::domains::auth::bot_signals::BotSignals;
use axum::http::{HeaderMap, HeaderValue};

#[test]
fn extracts_and_bounds_all_user_agent_client_hints() {
    let mut headers = HeaderMap::new();
    for (name, value) in [
        ("Sec-CH-UA", "\"Chromium\";v=\"126\""),
        ("Sec-CH-UA-Arch", "\"arm\""),
        ("Sec-CH-UA-Bitness", "\"64\""),
        ("Sec-CH-UA-Full-Version", "\"126.0.1.2\""),
        (
            "Sec-CH-UA-Full-Version-List",
            "\"Chromium\";v=\"126.0.1.2\"",
        ),
        ("Sec-CH-UA-Model", "\"Pixel 8\""),
        ("Sec-CH-UA-WoW64", "?0"),
        ("Sec-CH-UA-Form-Factors", "\"Mobile\""),
        ("Sec-CH-UA-Mobile", "?1"),
        ("Sec-CH-UA-Platform", "\"Android\""),
        ("Sec-CH-UA-Platform-Version", "\"14.0.0\""),
    ] {
        headers.insert(
            axum::http::HeaderName::from_bytes(name.as_bytes()).unwrap(),
            HeaderValue::from_static(value),
        );
    }

    let hints = UserAgentClientHints::from_headers(&headers);

    assert_eq!(hints.architecture.as_deref(), Some("\"arm\""));
    assert_eq!(hints.model.as_deref(), Some("\"Pixel 8\""));
    assert_eq!(hints.platform_version.as_deref(), Some("\"14.0.0\""));
}

#[test]
fn contradictory_platforms_raise_bot_risk() {
    let hints = UserAgentClientHints {
        platform: Some("\"Windows\"".to_string()),
        ..Default::default()
    };

    let assessment = hints.assess_consistency(
        Some("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)"),
        None,
        None,
    );

    assert_eq!(assessment.score, 0.20);
    assert_eq!(assessment.factors, ["ua_ch_legacy_platform_mismatch"]);
}

#[test]
fn matching_header_and_javascript_hints_do_not_raise_bot_risk() {
    let hints = UserAgentClientHints {
        brands: Some("\"Chromium\";v=\"126\"".to_string()),
        architecture: Some("\"arm\"".to_string()),
        bitness: Some("\"64\"".to_string()),
        full_version: Some("\"126.0.1.2\"".to_string()),
        full_version_list: Some("\"Chromium\";v=\"126.0.1.2\"".to_string()),
        model: Some("\"Pixel 8\"".to_string()),
        wow64: Some("?0".to_string()),
        form_factors: Some("\"Mobile\"".to_string()),
        mobile: Some("?1".to_string()),
        platform: Some("\"Android\"".to_string()),
        platform_version: Some("\"14.0.0\"".to_string()),
    };
    let signals = BotSignals {
        ua_brands: Some(vec!["Chromium/126".to_string()]),
        ua_architecture: Some("arm".to_string()),
        ua_bitness: Some("64".to_string()),
        ua_full_version_list: Some(vec!["Chromium/126.0.1.2".to_string()]),
        ua_form_factors: Some(vec!["Mobile".to_string()]),
        ua_model: Some("Pixel 8".to_string()),
        ua_mobile: Some(true),
        ua_platform: Some("Android".to_string()),
        ua_platform_version: Some("14.0.0".to_string()),
        ua_wow64: Some(false),
        ..Default::default()
    };

    let assessment = hints.assess_consistency(
        Some("Mozilla/5.0 (Linux; Android 10; Pixel 8) Chrome/126 Mobile"),
        Some(&signals),
        Some(&serde_json::json!({
            "platform": "android",
            "form_factor": "mobile"
        })),
    );

    assert_eq!(assessment.score, 0.0);
    assert!(assessment.factors.is_empty());
}
