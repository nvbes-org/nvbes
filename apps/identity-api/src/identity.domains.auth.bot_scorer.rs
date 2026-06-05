use serde_json::Value;

use super::bot_signals::BotSignals;
#[path = "identity.domains.auth.bot_scorer.tests.rs"]
mod tests;
#[path = "identity.domains.auth.bot_scorer.weights.rs"]
mod weights;

// ---------------------------------------------------------------------------
// Score result
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct BotScore {
    /// Normalized score in [0.0, 1.0]. Higher = more likely a bot.
    pub score: f64,
    /// Human-readable factors that contributed to the score.
    pub factors: Value,
}

impl BotScore {
    pub fn zero() -> Self {
        Self {
            score: 0.0,
            factors: serde_json::json!([]),
        }
    }
}

// ---------------------------------------------------------------------------
// Client signal scoring
// ---------------------------------------------------------------------------

/// Computes a bot score [0.0, 1.0] from client-submitted signals.
///
/// Absent signals are silently skipped — partial payloads are not penalised.
/// Only *present* signals with suspicious values contribute to the score.
pub fn score_client_signals(signals: &BotSignals) -> BotScore {
    let mut score = 0.0_f64;
    let mut factors: Vec<&str> = Vec::new();

    // --- Mouse entropy ---
    if let Some(count) = signals.mouse_event_count {
        let is_touch = signals.nav_touch_points.unwrap_or(0) > 0;
        if !is_touch && count == 0 {
            score += weights::W_MOUSE_ABSENT;
            factors.push("mouse_absent");
        }
    }
    if let Some(variance) = signals.mouse_variance {
        if variance < 0.01 && signals.mouse_event_count.unwrap_or(0) > 5 {
            score += weights::W_MOUSE_LINEAR;
            factors.push("mouse_linear");
        }
    }
    if let Some(path_len) = signals.mouse_path_length {
        let is_touch = signals.nav_touch_points.unwrap_or(0) > 0;
        if !is_touch && path_len < 10.0 && signals.mouse_event_count.unwrap_or(0) > 5 {
            score += weights::W_MOUSE_ABSENT;
            factors.push("mouse_no_path");
        }
    }

    // --- Keyboard biometrics ---
    if let Some(sample_count) = signals.kb_sample_count {
        if sample_count >= 3 {
            if let Some(dwell_var) = signals.kb_dwell_variance {
                if dwell_var < 5.0 {
                    score += weights::W_KB_ROBOTIC;
                    factors.push("kb_robotic_timing");
                }
            }
            if let Some(dwell_mean) = signals.kb_dwell_mean {
                if dwell_mean < 30.0 || dwell_mean > 500.0 {
                    score += weights::W_KB_ROBOTIC;
                    factors.push("kb_dwell_anomalous");
                }
            }
        }
    }

    // --- Scroll speed ---
    if let Some(speed_var) = signals.scroll_speed_variance {
        if speed_var == 0.0 && signals.scroll_event_count.unwrap_or(0) > 3 {
            score += weights::W_SCROLL_UNIFORM;
            factors.push("scroll_uniform_speed");
        }
    }

    // --- Visibility (IntersectionObserver) ---
    if let Some(false) = signals.submit_visible {
        score += weights::W_SUBMIT_HIDDEN;
        factors.push("submit_not_visible");
    }

    // --- Event cascade ---
    if let Some(cascade) = signals.event_cascade {
        let is_keyboard = signals.keyboard_submit.unwrap_or(false);
        if !is_keyboard && cascade < 0b11111 {
            score += weights::W_CASCADE_INCOMPLETE;
            factors.push("event_cascade_incomplete");
        }
    }

    // --- Event order ---
    if let Some(false) = signals.event_order_valid {
        score += weights::W_ORDER_INVALID;
        factors.push("event_order_invalid");
    }

    // --- Focus sequence ---
    if let Some(had_focus) = signals.email_had_focus {
        if !had_focus {
            score += weights::W_NO_FOCUS;
            factors.push("no_email_focus");
        }
    }

    // --- Speech synthesis ---
    // -1 means API unavailable (not scored). 0 means API available but no voices.
    if let Some(voices) = signals.speech_voices_count {
        if voices == 0 {
            score += weights::W_SPEECH_HEADLESS;
            factors.push("speech_no_voices");
        }
    }

    // --- Navigator consistency ---
    if let Some(touch_pts) = signals.nav_touch_points {
        // UA claims mobile but touch points = 0 → spoofed UA
        // We don't have the UA here — this is handled by http_signals.
        // Here we flag impossible hw_concurrency values.
        let _ = touch_pts; // used by http_signals scorer instead
    }
    if let Some(hw) = signals.nav_hw_concurrency {
        if hw == 0 || hw > 128 {
            score += weights::W_NAV_INCONSISTENT;
            factors.push("nav_hw_impossible");
        }
    }

    // --- Language / Timezone consistency ---
    check_tz_consistency(signals, &mut score, &mut factors);

    // --- Canvas fingerprint ---
    if let Some(hash) = &signals.canvas_hash {
        if weights::HEADLESS_CANVAS_HASHES.contains(&hash.as_str()) {
            score += weights::W_CANVAS_HEADLESS;
            factors.push("canvas_headless_hash");
        }
        // "unknown" hash is NOT scored — we only flag known headless signatures.
    }

    // --- WebGL renderer ---
    if let Some(renderer) = &signals.webgl_renderer {
        let r = renderer.to_lowercase();
        if weights::HEADLESS_WEBGL_RENDERERS
            .iter()
            .any(|h| r.contains(h))
        {
            score += weights::W_WEBGL_HEADLESS;
            factors.push("webgl_headless_renderer");
        }
    }

    // --- Font count ---
    if let Some(font_count) = signals.font_count {
        if font_count <= 3 {
            score += weights::W_FONT_MINIMAL;
            factors.push("font_minimal");
        }
    }

    // --- Known Automation & Stealth Bypasses ---
    if let Some(true) = signals.auto_webdriver {
        score += weights::W_AUTO_WEBDRIVER;
        factors.push("auto_webdriver_active");
    }
    if let Some(true) = signals.auto_webdriver_spoofed {
        score += weights::W_AUTO_WEBDRIVER_SPOOFED;
        factors.push("auto_webdriver_spoofed");
    }
    if let Some(true) = signals.auto_chrome_driver_injected {
        score += weights::W_AUTO_CHROME_DRIVER;
        factors.push("auto_chromedriver_detected");
    }
    if let Some(true) = signals.auto_global_tools_detected {
        score += weights::W_AUTO_GLOBAL_TOOLS;
        factors.push("auto_global_tools_detected");
    }
    if let Some(true) = signals.auto_chrome_runtime_missing {
        score += weights::W_AUTO_CHROME_RUNTIME_MISSING;
        factors.push("auto_chrome_runtime_missing");
    }
    if let Some(true) = signals.auto_native_function_spoofed {
        score += weights::W_AUTO_NATIVE_SPOOFED;
        factors.push("auto_native_function_spoofed");
    }
    if let Some(true) = signals.auto_plugins_inconsistent {
        score += weights::W_AUTO_PLUGINS_INCONSISTENT;
        factors.push("auto_plugins_inconsistent");
    }
    if let Some(true) = signals.auto_iframe_webdriver {
        score += weights::W_AUTO_IFRAME_WEBDRIVER;
        factors.push("auto_iframe_webdriver_leaked");
    }
    if let Some(true) = signals.auto_ua_data_inconsistent {
        score += weights::W_AUTO_UA_DATA_INCONSISTENT;
        factors.push("auto_ua_data_inconsistent");
    }
    if let Some(true) = signals.auto_chrome_spoofed {
        score += weights::W_AUTO_CHROME_SPOOFED;
        factors.push("auto_chrome_spoofed");
    }

    BotScore {
        score: score.min(1.0),
        factors: serde_json::json!(factors),
    }
}

// ---------------------------------------------------------------------------
// Timezone consistency helper
// ---------------------------------------------------------------------------

fn check_tz_consistency(signals: &BotSignals, score: &mut f64, factors: &mut Vec<&'static str>) {
    let (Some(tz), Some(offset)) = (&signals.tz, signals.tz_offset) else {
        return;
    };

    // Compute expected UTC offset in minutes for common timezones.
    // This is a simplified check — full IANA database would be more accurate.
    let expected_offset: Option<i32> = match tz.as_str() {
        "Europe/Paris" | "Europe/Berlin" | "Europe/Rome" | "Europe/Madrid" => {
            // CET = UTC+1 (offset = -60), CEST = UTC+2 (offset = -120)
            // getTimezoneOffset() returns the *negative* of the UTC offset.
            Some(-60) // Allow both -60 and -120 for DST
        }
        "America/New_York" | "America/Toronto" => Some(300), // EST = UTC-5
        "America/Los_Angeles" | "America/Vancouver" => Some(480), // PST = UTC-8
        "Europe/London" => Some(0),                          // GMT (simplified, ignores BST)
        "Asia/Tokyo" => Some(-540),                          // JST = UTC+9
        "UTC" | "Etc/UTC" => Some(0),
        _ => None, // Unknown timezone — don't score
    };

    if let Some(expected) = expected_offset {
        // Allow ±60min tolerance for DST
        let diff = (offset - expected).abs();
        if diff > 60 {
            *score += weights::W_TZ_MISMATCH;
            factors.push("tz_offset_mismatch");
        }
    }
}
