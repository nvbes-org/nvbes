use axum::http::HeaderMap;
use serde_json::{Map, Value};

const MAX_HINT_LENGTH: usize = 512;

#[path = "identity.domains.auth.ua_client_hints.assessment.rs"]
mod assessment;
pub use assessment::UserAgentClientHintAssessment;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UserAgentClientHints {
    pub brands: Option<String>,
    pub architecture: Option<String>,
    pub bitness: Option<String>,
    pub full_version: Option<String>,
    pub full_version_list: Option<String>,
    pub model: Option<String>,
    pub wow64: Option<String>,
    pub form_factors: Option<String>,
    pub mobile: Option<String>,
    pub platform: Option<String>,
    pub platform_version: Option<String>,
}

impl UserAgentClientHints {
    pub fn from_headers(headers: &HeaderMap) -> Self {
        Self {
            brands: header_value(headers, "Sec-CH-UA"),
            architecture: header_value(headers, "Sec-CH-UA-Arch"),
            bitness: header_value(headers, "Sec-CH-UA-Bitness"),
            full_version: header_value(headers, "Sec-CH-UA-Full-Version"),
            full_version_list: header_value(headers, "Sec-CH-UA-Full-Version-List"),
            model: header_value(headers, "Sec-CH-UA-Model"),
            wow64: header_value(headers, "Sec-CH-UA-WoW64"),
            form_factors: header_value(headers, "Sec-CH-UA-Form-Factors"),
            mobile: header_value(headers, "Sec-CH-UA-Mobile"),
            platform: header_value(headers, "Sec-CH-UA-Platform"),
            platform_version: header_value(headers, "Sec-CH-UA-Platform-Version"),
        }
    }

    pub fn from_session(session: &nvbes_redis::session::CachedSession) -> Self {
        Self {
            brands: session.sec_ch_ua.clone(),
            architecture: session.sec_ch_ua_arch.clone(),
            bitness: session.sec_ch_ua_bitness.clone(),
            full_version: session.sec_ch_ua_full_version.clone(),
            full_version_list: session.sec_ch_ua_full_version_list.clone(),
            model: session.sec_ch_ua_model.clone(),
            wow64: session.sec_ch_ua_wow64.clone(),
            form_factors: session.sec_ch_ua_form_factors.clone(),
            mobile: session.sec_ch_ua_mobile.clone(),
            platform: session.sec_ch_ua_platform.clone(),
            platform_version: session.sec_ch_ua_platform_version.clone(),
        }
    }

    pub fn apply_to_session(&self, session: &mut nvbes_redis::session::CachedSession) {
        session.sec_ch_ua = self.brands.clone();
        session.sec_ch_ua_arch = self.architecture.clone();
        session.sec_ch_ua_bitness = self.bitness.clone();
        session.sec_ch_ua_full_version = self.full_version.clone();
        session.sec_ch_ua_full_version_list = self.full_version_list.clone();
        session.sec_ch_ua_model = self.model.clone();
        session.sec_ch_ua_wow64 = self.wow64.clone();
        session.sec_ch_ua_form_factors = self.form_factors.clone();
        session.sec_ch_ua_mobile = self.mobile.clone();
        session.sec_ch_ua_platform = self.platform.clone();
        session.sec_ch_ua_platform_version = self.platform_version.clone();
    }

    pub fn enrich_device_profile(&self, profile: Option<Value>) -> Option<Value> {
        let mut object = match profile {
            Some(Value::Object(object)) => object,
            Some(_) => return None,
            None if self.is_empty() => return None,
            None => Map::new(),
        };
        object.insert("version".to_string(), Value::from(2));
        insert_string(&mut object, "ua_architecture", self.architecture.as_deref());
        insert_string(&mut object, "ua_bitness", self.bitness.as_deref());
        insert_string(&mut object, "ua_model", self.model.as_deref());
        insert_string(&mut object, "ua_platform", self.platform.as_deref());
        insert_string(&mut object, "ua_form_factors", self.form_factors.as_deref());
        if let Some(value) = structured_bool(self.wow64.as_deref()) {
            object.insert("ua_wow64".to_string(), Value::Bool(value));
        }
        Some(Value::Object(object))
    }

    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

fn header_value(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.chars().take(MAX_HINT_LENGTH).collect())
}

fn insert_string(object: &mut Map<String, Value>, key: &str, value: Option<&str>) {
    if let Some(value) = structured_string(value).filter(|value| !value.is_empty()) {
        object.insert(key.to_string(), Value::String(value));
    }
}

pub(super) fn structured_string(value: Option<&str>) -> Option<String> {
    let value = value?.trim();
    Some(
        value
            .strip_prefix('"')
            .and_then(|value| value.strip_suffix('"'))
            .unwrap_or(value)
            .replace("\\\"", "\"")
            .replace("\\\\", "\\"),
    )
}

pub(super) fn structured_bool(value: Option<&str>) -> Option<bool> {
    match value?.trim() {
        "?1" => Some(true),
        "?0" => Some(false),
        _ => None,
    }
}

pub(super) fn brand_names(value: &str) -> Vec<String> {
    value
        .split(',')
        .filter_map(|item| structured_string(item.split(';').next()))
        .filter(|brand| !brand.is_empty())
        .collect()
}

pub(super) fn brand_versions(value: &str) -> Vec<String> {
    value
        .split(',')
        .filter_map(|item| {
            let (brand, version) = item.split_once(';')?;
            let brand = structured_string(Some(brand))?;
            let version = version
                .split_once('=')
                .and_then(|(_, value)| structured_string(Some(value)))?;
            Some(format!("{brand}/{version}"))
        })
        .collect()
}

pub(super) fn quoted_list_values(value: &str) -> Vec<String> {
    value
        .split(',')
        .filter_map(|value| structured_string(Some(value)))
        .collect()
}

#[cfg(test)]
#[path = "identity.domains.auth.ua_client_hints.tests.rs"]
mod tests;
