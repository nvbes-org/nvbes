use super::{parse, risk, stable_family};
use crate::domains::auth::bot_signals::BotSignals;

const IPHONE_SAFARI: &str = "Mozilla/5.0 (iPhone; CPU iPhone OS 17_5 like Mac OS X) \
AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.5 Mobile/15E148 Safari/604.1";
const ANDROID_CHROME: &str = "Mozilla/5.0 (Linux; Android 14; Pixel 8 Pro) \
AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Mobile Safari/537.36";

#[test]
fn parses_browser_os_and_device() {
    let safari = parse(Some(IPHONE_SAFARI), None).expect("Safari should parse");
    assert_eq!(safari.browser.as_deref(), Some("Mobile Safari"));
    assert_eq!(safari.browser_version, Some(17.5));
    assert_eq!(safari.os.as_deref(), Some("iOS"));
    assert_eq!(safari.os_version.as_deref(), Some("17.5.0"));
    assert_eq!(safari.device.as_deref(), Some("iPhone"));
    assert_eq!(safari.device_type, "mobile");

    let chrome = parse(Some(ANDROID_CHROME), None).expect("Chrome should parse");
    assert_eq!(chrome.browser.as_deref(), Some("Chrome"));
    assert_eq!(chrome.os_version.as_deref(), Some("14.0.0"));
}

#[test]
fn browser_hints_identify_brave() {
    let client = parse(Some(ANDROID_CHROME), Some("\"Brave\";v=\"126\"")).unwrap();
    assert_eq!(client.browser.as_deref(), Some("Brave"));
}

#[test]
fn stable_family_ignores_browser_version_updates() {
    let previous = stable_family(
        Some("Mozilla/5.0 (Windows NT 10.0) Chrome/126.0 Safari/537.36"),
        None,
    );
    let current = stable_family(
        Some("Mozilla/5.0 (Windows NT 10.0) Chrome/127.0 Safari/537.36"),
        None,
    );
    assert_eq!(previous, current);
}

#[test]
fn automation_and_client_contradictions_raise_risk() {
    let signals = BotSignals {
        nav_touch_points: Some(0),
        ua_platform: Some("Windows".to_string()),
        ua_mobile: Some(false),
        ..BotSignals::default()
    };
    let assessment = risk::assess(
        Some("Mozilla/5.0 (Linux; Android 14) HeadlessChrome/126.0 Mobile Safari/537.36"),
        None,
        Some(&signals),
    );

    assert_eq!(assessment.score, 1.0);
    assert!(assessment.factors.contains(&"ua_automation_signature"));
    assert!(assessment.factors.contains(&"ua_touch_capability_mismatch"));
    assert!(assessment.factors.contains(&"ua_js_platform_mismatch"));
}
