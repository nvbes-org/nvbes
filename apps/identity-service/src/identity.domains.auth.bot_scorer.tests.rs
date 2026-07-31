#[cfg(test)]
use super::super::bot_scorer::score_client_signals;
#[cfg(test)]
use super::super::bot_signals::BotSignals;
#[cfg(test)]
use super::weights::*;

#[cfg(test)]
fn empty() -> BotSignals {
    BotSignals::default()
}

#[cfg(test)]
#[test]
fn empty_signals_score_zero() {
    let score = score_client_signals(&empty());
    assert_eq!(score.score, 0.0);
}

#[cfg(test)]
#[test]
fn robotic_keyboard_scores() {
    let s = BotSignals {
        kb_dwell_variance: Some(0.5),
        kb_sample_count: Some(5),
        ..empty()
    };
    let score = score_client_signals(&s);
    assert!(score.score >= W_KB_ROBOTIC);
}

#[cfg(test)]
#[test]
fn headless_webgl_scores() {
    let s = BotSignals {
        webgl_renderer: Some("ANGLE (SwiftShader renderer)".to_string()),
        ..empty()
    };
    let score = score_client_signals(&s);
    assert!(score.score >= W_WEBGL_HEADLESS);
}

#[cfg(test)]
#[test]
fn incomplete_cascade_scores() {
    let s = BotSignals {
        event_cascade: Some(0b00001),
        keyboard_submit: Some(false),
        ..empty()
    };
    let score = score_client_signals(&s);
    assert!(score.score >= W_CASCADE_INCOMPLETE);
}

#[cfg(test)]
#[test]
fn touch_device_mouse_absent_not_scored() {
    let s = BotSignals {
        mouse_event_count: Some(0),
        nav_touch_points: Some(5),
        ..empty()
    };
    let score = score_client_signals(&s);
    assert!(score.score < W_MOUSE_ABSENT);
}

#[cfg(test)]
#[test]
fn automation_signals_scores_highly() {
    let s1 = BotSignals {
        auto_webdriver_spoofed: Some(true),
        ..empty()
    };
    assert!(score_client_signals(&s1).score >= W_AUTO_WEBDRIVER_SPOOFED);

    let s2 = BotSignals {
        auto_chrome_driver_injected: Some(true),
        ..empty()
    };
    assert!(score_client_signals(&s2).score >= W_AUTO_CHROME_DRIVER);

    let s3 = BotSignals {
        auto_global_tools_detected: Some(true),
        ..empty()
    };
    assert!(score_client_signals(&s3).score >= W_AUTO_GLOBAL_TOOLS);

    let s4 = BotSignals {
        auto_native_function_spoofed: Some(true),
        ..empty()
    };
    assert!(score_client_signals(&s4).score >= W_AUTO_NATIVE_SPOOFED);

    let s5 = BotSignals {
        auto_iframe_webdriver: Some(true),
        ..empty()
    };
    assert!(score_client_signals(&s5).score >= W_AUTO_IFRAME_WEBDRIVER);

    let s6 = BotSignals {
        auto_ua_data_inconsistent: Some(true),
        ..empty()
    };
    assert!(score_client_signals(&s6).score >= W_AUTO_UA_DATA_INCONSISTENT);

    let s7 = BotSignals {
        auto_chrome_spoofed: Some(true),
        ..empty()
    };
    assert!(score_client_signals(&s7).score >= W_AUTO_CHROME_SPOOFED);
}

#[cfg(test)]
#[test]
fn score_capped_at_one() {
    let s = BotSignals {
        mouse_event_count: Some(0),
        mouse_variance: Some(0.0),
        kb_dwell_variance: Some(0.0),
        kb_sample_count: Some(10),
        event_cascade: Some(0),
        keyboard_submit: Some(false),
        email_had_focus: Some(false),
        speech_voices_count: Some(0),
        nav_hw_concurrency: Some(0),
        webgl_renderer: Some("SwiftShader".to_string()),
        canvas_hash: Some(HEADLESS_CANVAS_HASHES[0].to_string()),
        font_count: Some(0),
        nav_touch_points: Some(0),
        auto_webdriver_spoofed: Some(true),
        auto_chrome_driver_injected: Some(true),
        ..empty()
    };
    let score = score_client_signals(&s);
    assert!(score.score <= 1.0);
}
