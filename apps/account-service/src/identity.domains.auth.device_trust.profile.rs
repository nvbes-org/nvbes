use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CoarseDeviceProfile {
    pub version: u8,
    pub platform: Option<String>,
    pub form_factor: Option<String>,
    pub cpu_bucket: Option<u8>,
    pub memory_bucket: Option<u8>,
    pub touch_capable: Option<bool>,
    pub color_depth_bucket: Option<u8>,
    pub screen_bucket: Option<String>,
    pub timezone_offset_bucket: Option<i16>,
    pub ua_architecture: Option<String>,
    pub ua_bitness: Option<String>,
    pub ua_form_factor: Option<String>,
    pub ua_model: Option<String>,
    pub ua_platform: Option<String>,
    pub ua_wow64: Option<bool>,
}

impl CoarseDeviceProfile {
    pub fn from_value(value: &Value) -> Option<Self> {
        let object = value.as_object()?;
        Some(Self {
            version: object
                .get("version")
                .and_then(Value::as_u64)
                .filter(|value| (1..=2).contains(value))
                .unwrap_or(1) as u8,
            platform: normalized_choice(
                object.get("platform").and_then(Value::as_str),
                &["android", "ios", "linux", "macos", "windows", "other"],
            ),
            form_factor: normalized_choice(
                object.get("form_factor").and_then(Value::as_str),
                &["desktop", "mobile", "tablet", "other"],
            ),
            cpu_bucket: allowed_u8(object.get("cpu_bucket"), &[1, 2, 4, 8, 16]),
            memory_bucket: allowed_u8(object.get("memory_bucket"), &[1, 2, 4, 8]),
            touch_capable: object.get("touch_capable").and_then(Value::as_bool),
            color_depth_bucket: allowed_u8(object.get("color_depth_bucket"), &[16, 24, 30, 32]),
            screen_bucket: normalized_choice(
                object.get("screen_bucket").and_then(Value::as_str),
                &["compact", "medium", "large", "wide"],
            ),
            timezone_offset_bucket: object
                .get("timezone_offset_bucket")
                .and_then(Value::as_i64)
                .filter(|value| (-14..=14).contains(value))
                .map(|value| value as i16),
            ua_architecture: normalized_choice(
                object.get("ua_architecture").and_then(Value::as_str),
                &["arm", "x86"],
            ),
            ua_bitness: normalized_choice(
                object.get("ua_bitness").and_then(Value::as_str),
                &["32", "64"],
            ),
            ua_form_factor: normalized_form_factor(
                object.get("ua_form_factors").and_then(Value::as_str),
            ),
            ua_model: normalized_model(object.get("ua_model").and_then(Value::as_str)),
            ua_platform: normalized_choice(
                object.get("ua_platform").and_then(Value::as_str),
                &[
                    "android",
                    "chrome os",
                    "chromium os",
                    "ios",
                    "linux",
                    "macos",
                    "windows",
                ],
            ),
            ua_wow64: object.get("ua_wow64").and_then(Value::as_bool),
        })
    }

    pub fn keyed_hash(&self, secret: &str) -> String {
        let serialized = serde_json::to_vec(self).unwrap_or_default();
        keyed_hash(secret, b"nvbes.device-profile.v1", &serialized)
    }
}

pub fn installation_token_hash(secret: &str, token: &str) -> String {
    keyed_hash(secret, b"nvbes.device-installation.v1", token.as_bytes())
}

pub fn network_hash(secret: &str, ip: Option<&str>) -> Option<String> {
    let marker = network_marker(ip?)?;
    Some(keyed_hash(
        secret,
        b"nvbes.device-network.v1",
        marker.as_bytes(),
    ))
}

fn keyed_hash(secret: &str, domain: &[u8], value: &[u8]) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .expect("HMAC accepts signing keys of any length");
    mac.update(domain);
    mac.update(&[0]);
    mac.update(value);
    hex::encode(mac.finalize().into_bytes())
}

fn normalized_choice(value: Option<&str>, allowed: &[&str]) -> Option<String> {
    let normalized = value?.trim().to_ascii_lowercase();
    allowed.contains(&normalized.as_str()).then_some(normalized)
}

fn allowed_u8(value: Option<&Value>, allowed: &[u8]) -> Option<u8> {
    let value = value?.as_u64().and_then(|value| u8::try_from(value).ok())?;
    allowed.contains(&value).then_some(value)
}

fn normalized_form_factor(value: Option<&str>) -> Option<String> {
    let value = value?.trim().to_ascii_lowercase();
    [
        "desktop",
        "mobile",
        "tablet",
        "automotive",
        "xr",
        "eink",
        "watch",
    ]
    .into_iter()
    .find(|candidate| value.contains(candidate))
    .map(str::to_string)
}

fn normalized_model(value: Option<&str>) -> Option<String> {
    let value = value?.trim().to_ascii_lowercase();
    (!value.is_empty()).then(|| value.chars().take(128).collect())
}

fn network_marker(value: &str) -> Option<String> {
    if value.contains(':') {
        let groups = value.split(':').take(4).collect::<Vec<_>>();
        return (!groups.is_empty()).then(|| groups.join(":"));
    }
    let octets = value.split('.').take(3).collect::<Vec<_>>();
    (octets.len() == 3).then(|| octets.join("."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_ignores_high_entropy_and_unknown_fields() {
        let profile = CoarseDeviceProfile::from_value(&serde_json::json!({
            "platform": "macOS",
            "cpu_bucket": 8,
            "canvas_hash": "must-not-be-retained",
            "webgl_renderer": "must-not-be-retained"
        }))
        .expect("profile should parse");

        let serialized = serde_json::to_string(&profile).expect("profile should serialize");
        assert_eq!(profile.platform.as_deref(), Some("macos"));
        assert!(!serialized.contains("canvas"));
        assert!(!serialized.contains("webgl"));
    }

    #[test]
    fn profile_hash_uses_stable_ua_client_hints() {
        let first = CoarseDeviceProfile::from_value(&serde_json::json!({
            "version": 2,
            "ua_architecture": "arm",
            "ua_bitness": "64",
            "ua_model": "Pixel 8",
            "ua_platform": "android",
            "ua_form_factors": "Mobile",
            "ua_wow64": false,
            "ua_platform_version": "14.0.0",
            "ua_full_version": "126.0.1.2"
        }))
        .unwrap();
        let second = CoarseDeviceProfile::from_value(&serde_json::json!({
            "version": 2,
            "ua_architecture": "arm",
            "ua_bitness": "64",
            "ua_model": "Pixel 8",
            "ua_platform": "android",
            "ua_form_factors": "Mobile",
            "ua_wow64": false,
            "ua_platform_version": "15.0.0",
            "ua_full_version": "127.0.0.1"
        }))
        .unwrap();

        assert_eq!(first.keyed_hash("secret"), second.keyed_hash("secret"));
    }

    #[test]
    fn keyed_hashes_are_domain_separated() {
        let token_hash = installation_token_hash("secret", "same-value");
        let profile = CoarseDeviceProfile::from_value(&serde_json::json!({})).unwrap();
        assert_ne!(token_hash, profile.keyed_hash("secret"));
    }
}
