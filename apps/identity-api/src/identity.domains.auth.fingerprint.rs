use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeviceFingerprint {
    pub browser: Option<String>,
    pub os: Option<String>,
    pub device_type: Option<String>,
    pub visitor_id: Option<String>,
    pub canvas_hash: Option<String>,
    pub webgl_hash: Option<String>,
    /// Raw WebGL vendor string (e.g. "Google Inc. (NVIDIA)").
    pub webgl_vendor: Option<String>,
    /// Raw WebGL renderer string (e.g. "ANGLE (NVIDIA, ...)").
    pub webgl_renderer: Option<String>,
    pub ip_address: Option<String>,
    /// Number of system fonts available out of a fixed test set of 20.
    pub font_count: Option<u32>,
    pub nav_touch_points: Option<u32>,
    pub nav_hw_concurrency: Option<u32>,
}

const HEADLESS_RENDERERS: &[&str] = &["swiftshader", "llvmpipe", "softpipe"];

impl DeviceFingerprint {
    pub fn is_suspicious(&self) -> bool {
        if self.browser.as_deref() == Some("HeadlessChrome") {
            return true;
        }
        if let Some(renderer) = &self.webgl_renderer {
            let r = renderer.to_lowercase();
            if HEADLESS_RENDERERS.iter().any(|h| r.contains(h)) {
                return true;
            }
        }
        false
    }
}
