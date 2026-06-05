pub const HEADLESS_CANVAS_HASHES: &[&str] = &[
    "8c3bb7d41a6af3ade4f9ec5aa0e39f6b19a1f8b2c5d3e7a9b4c6d8e2f1a3b5c7",
    "a1b2c3d4e5f6789012345678901234567890abcdefabcdefabcdefabcdefabcdef",
];

pub const HEADLESS_WEBGL_RENDERERS: &[&str] = &["swiftshader", "llvmpipe", "softpipe"];

pub const W_MOUSE_ABSENT: f64 = 0.08;
pub const W_MOUSE_LINEAR: f64 = 0.10;
pub const W_KB_ROBOTIC: f64 = 0.18;
pub const W_SCROLL_UNIFORM: f64 = 0.06;
pub const W_CASCADE_INCOMPLETE: f64 = 0.20;
pub const W_ORDER_INVALID: f64 = 0.25;
pub const W_SUBMIT_HIDDEN: f64 = 0.40;
pub const W_NO_FOCUS: f64 = 0.08;
pub const W_SPEECH_HEADLESS: f64 = 0.08;
pub const W_NAV_INCONSISTENT: f64 = 0.15;
pub const W_TZ_MISMATCH: f64 = 0.10;
pub const W_CANVAS_HEADLESS: f64 = 0.20;
pub const W_WEBGL_HEADLESS: f64 = 0.20;
pub const W_FONT_MINIMAL: f64 = 0.08;
pub const W_AUTO_WEBDRIVER: f64 = 0.90;
pub const W_AUTO_WEBDRIVER_SPOOFED: f64 = 0.95;
pub const W_AUTO_CHROME_DRIVER: f64 = 0.95;
pub const W_AUTO_GLOBAL_TOOLS: f64 = 0.95;
pub const W_AUTO_CHROME_RUNTIME_MISSING: f64 = 0.30;
pub const W_AUTO_NATIVE_SPOOFED: f64 = 0.95;
pub const W_AUTO_PLUGINS_INCONSISTENT: f64 = 0.25;
pub const W_AUTO_IFRAME_WEBDRIVER: f64 = 0.95;
pub const W_AUTO_UA_DATA_INCONSISTENT: f64 = 0.80;
pub const W_AUTO_CHROME_SPOOFED: f64 = 0.90;
