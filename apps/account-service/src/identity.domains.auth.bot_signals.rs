use serde::Deserialize;
use utoipa::ToSchema;

/// Client-collected signals for bot scoring at the `/challenge/identifier` step.
///
/// All fields are `Option` — absent fields are silently skipped during scoring.
/// The struct is designed so that a minimal client (curl, legacy SDK) can omit
/// everything without being penalised: only _present_ signals contribute to the score.
#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct BotSignals {
    // ---- Behavioral --------------------------------------------------------
    /// Number of mousemove events recorded before form submission.
    pub mouse_event_count: Option<u32>,

    /// Variance of mouse acceleration vectors (higher = more human-like).
    pub mouse_variance: Option<f64>,

    /// Total euclidean distance travelled by the mouse cursor in pixels.
    pub mouse_path_length: Option<f64>,

    /// Variance of key dwell times in milliseconds (keydown → keyup duration).
    pub kb_dwell_variance: Option<f64>,

    /// Mean of key dwell times in milliseconds.
    pub kb_dwell_mean: Option<f64>,

    /// Variance of key flight times in milliseconds (keyup → next keydown).
    pub kb_flight_variance: Option<f64>,

    /// Mean of key flight times in milliseconds.
    pub kb_flight_mean: Option<f64>,

    /// Number of keystroke pairs sampled for biometrics.
    pub kb_sample_count: Option<u32>,

    /// Bitfield of observed submit events (5 bits):
    /// bit 0 = pointerdown, 1 = mousedown, 2 = pointerup, 3 = mouseup, 4 = click.
    pub event_cascade: Option<u8>,

    /// True if events arrived in the correct order (pointerdown → click).
    /// None if no pointer events were observed.
    pub event_order_valid: Option<bool>,

    /// True if the form was submitted via keyboard (Enter), not a click.
    /// Used to adjust event_cascade expectations.
    pub keyboard_submit: Option<bool>,

    /// True if the email field received a focusin event before form submission.
    pub email_had_focus: Option<bool>,

    /// True if focus occurred before the email field was populated.
    pub email_focus_before_value: Option<bool>,

    /// True if the caret was positioned at the end of the email string.
    pub caret_at_end: Option<bool>,

    /// Number of scroll events observed on the login page.
    pub scroll_event_count: Option<u32>,

    /// Variance of scroll speed (pixels per millisecond) across all scroll events.
    pub scroll_speed_variance: Option<f64>,

    /// True if the submit button was visible (IntersectionObserver).
    /// None if IntersectionObserver is unsupported.
    pub submit_visible: Option<bool>,

    // ---- Environment -------------------------------------------------------
    /// Number of voices returned by `speechSynthesis.getVoices()`. -1 = unavailable.
    pub speech_voices_count: Option<i32>,

    /// Value of `navigator.maxTouchPoints`.
    pub nav_touch_points: Option<u32>,

    /// Value of `navigator.hardwareConcurrency`.
    pub nav_hw_concurrency: Option<u32>,

    /// Value of `navigator.deviceMemory` in GB.
    pub nav_device_memory: Option<f64>,

    /// Value of `navigator.language` (e.g. "fr-FR").
    pub lang: Option<String>,

    /// Resolved IANA timezone (e.g. "Europe/Paris").
    pub tz: Option<String>,

    /// Value of `new Date().getTimezoneOffset()` in minutes.
    pub tz_offset: Option<i32>,

    /// State of the notifications permission ("granted"/"denied"/"prompt"/"unavailable").
    pub perm_notifications: Option<String>,

    /// Variance of requestAnimationFrame deltas in milliseconds.
    pub raf_variance: Option<f64>,

    /// Mean requestAnimationFrame interval in milliseconds.
    pub raf_mean: Option<f64>,

    /// Whether `navigator.getBattery()` is available.
    pub battery_available: Option<bool>,

    /// True if localStorage is readable and writable.
    pub storage_ok: Option<bool>,

    // ---- Fingerprint -------------------------------------------------------
    /// SHA-256 hex of a deterministic canvas draw. Used for headless detection.
    pub canvas_hash: Option<String>,

    /// Raw `UNMASKED_VENDOR_WEBGL` string (e.g. "Google Inc. (NVIDIA)").
    pub webgl_vendor: Option<String>,

    /// Raw `UNMASKED_RENDERER_WEBGL` string (e.g. "ANGLE (NVIDIA, ...)").
    pub webgl_renderer: Option<String>,

    /// Number of system fonts available (out of a fixed test set of 20).
    pub font_count: Option<u32>,

    /// screen.width in pixels.
    pub screen_width: Option<u32>,

    /// screen.height in pixels.
    pub screen_height: Option<u32>,

    /// screen.colorDepth in bits.
    pub color_depth: Option<u32>,

    /// window.devicePixelRatio.
    pub pixel_ratio: Option<f64>,

    /// navigator.platform (e.g. "MacIntel", "Win32", "Linux x86_64").
    pub platform: Option<String>,

    /// AudioContext oscilloscope hash — hardware-dependent float.
    pub audio_hash: Option<String>,

    // ---- Automation --------------------------------------------------------
    /// Value of navigator.webdriver.
    pub auto_webdriver: Option<bool>,

    /// True if navigator.webdriver getter signature has been spoofed.
    pub auto_webdriver_spoofed: Option<bool>,

    /// True if chromedriver signature properties were found on window/document.
    pub auto_chrome_driver_injected: Option<bool>,

    /// True if global variables injected by Selenium, Playwright, Cypress, etc. were found.
    pub auto_global_tools_detected: Option<bool>,

    /// True if chrome object is present but chrome.runtime is missing (headless artifact).
    pub auto_chrome_runtime_missing: Option<bool>,

    /// True if common native functions like permissions.query have spoofed toString signatures.
    pub auto_native_function_spoofed: Option<bool>,

    /// True if navigator.plugins is inconsistent (e.g. empty on desktop or fake instance).
    pub auto_plugins_inconsistent: Option<bool>,

    /// True if a dynamically spawned clean iframe leaks navigator.webdriver === true.
    pub auto_iframe_webdriver: Option<bool>,

    /// True if navigator.userAgentData and User-Agent headers contradict each other.
    pub auto_ua_data_inconsistent: Option<bool>,

    /// True if window.chrome is absent or spoofed with non-native descriptors on Chrome UA.
    pub auto_chrome_spoofed: Option<bool>,
}
