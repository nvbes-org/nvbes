use serde_json::Value;

use super::{
    UserAgentClientHints, brand_names, brand_versions, quoted_list_values, structured_bool,
    structured_string,
};
use crate::domains::auth::bot_signals::BotSignals;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct UserAgentClientHintAssessment {
    pub score: f64,
    pub factors: Vec<&'static str>,
}

impl UserAgentClientHints {
    pub fn assess_consistency(
        &self,
        user_agent: Option<&str>,
        signals: Option<&BotSignals>,
        device_profile: Option<&Value>,
    ) -> UserAgentClientHintAssessment {
        let mut assessment = UserAgentClientHintAssessment::default();
        let platform = normalized_platform(self.platform.as_deref());
        let mobile = structured_bool(self.mobile.as_deref());
        let form_factor = normalized_form_factor(self.form_factors.as_deref());

        compare_legacy_user_agent(&mut assessment, platform.as_deref(), mobile, user_agent);
        compare_header_invariants(&mut assessment, self);
        if let Some(signals) = signals {
            compare_javascript_hints(&mut assessment, self, platform.clone(), mobile, signals);
        }
        if let Some(object) = device_profile.and_then(Value::as_object) {
            compare_device_profile(&mut assessment, platform, form_factor, object);
        }

        assessment.score = assessment.score.min(0.60);
        assessment
    }
}

impl UserAgentClientHintAssessment {
    fn add(&mut self, score: f64, factor: &'static str) {
        self.score += score;
        self.factors.push(factor);
    }
}

fn compare_legacy_user_agent(
    assessment: &mut UserAgentClientHintAssessment,
    platform: Option<&str>,
    mobile: Option<bool>,
    user_agent: Option<&str>,
) {
    if let (Some(expected), Some(legacy)) = (platform, legacy_platform(user_agent))
        && expected != legacy
    {
        assessment.add(0.20, "ua_ch_legacy_platform_mismatch");
    }
    if let (Some(expected), Some(legacy)) = (mobile, legacy_mobile(user_agent))
        && expected != legacy
    {
        assessment.add(0.12, "ua_ch_legacy_mobile_mismatch");
    }
}

fn compare_header_invariants(
    assessment: &mut UserAgentClientHintAssessment,
    hints: &UserAgentClientHints,
) {
    if let (Some(brands), Some(full_versions)) = (&hints.brands, &hints.full_version_list)
        && !brand_names(brands)
            .iter()
            .any(|brand| full_versions.contains(brand))
    {
        assessment.add(0.15, "ua_ch_brand_list_mismatch");
    }
    if let (Some(version), Some(full_versions)) = (
        structured_string(hints.full_version.as_deref()),
        &hints.full_version_list,
    ) && !version.is_empty()
        && !full_versions.contains(&version)
    {
        assessment.add(0.10, "ua_ch_full_version_mismatch");
    }
}

fn compare_javascript_hints(
    assessment: &mut UserAgentClientHintAssessment,
    hints: &UserAgentClientHints,
    platform: Option<String>,
    mobile: Option<bool>,
    signals: &BotSignals,
) {
    compare_list(
        assessment,
        hints.brands.as_deref().map(brand_versions),
        signals.ua_brands.as_deref(),
        "ua_ch_js_brands_mismatch",
    );
    for (header, client, factor) in [
        (
            normalized_token(hints.architecture.as_deref()),
            normalized_token(signals.ua_architecture.as_deref()),
            "ua_ch_js_architecture_mismatch",
        ),
        (
            normalized_token(hints.bitness.as_deref()),
            normalized_token(signals.ua_bitness.as_deref()),
            "ua_ch_js_bitness_mismatch",
        ),
        (
            platform,
            signals.ua_platform.as_deref().map(normalize_platform_value),
            "ua_ch_js_platform_mismatch",
        ),
        (
            normalized_token(hints.model.as_deref()),
            normalized_token(signals.ua_model.as_deref()),
            "ua_ch_js_model_mismatch",
        ),
        (
            normalized_token(hints.platform_version.as_deref()),
            normalized_token(signals.ua_platform_version.as_deref()),
            "ua_ch_js_platform_version_mismatch",
        ),
    ] {
        compare_string(assessment, header, client, factor);
    }
    compare_list(
        assessment,
        hints.form_factors.as_deref().map(quoted_list_values),
        signals.ua_form_factors.as_deref(),
        "ua_ch_js_form_factors_mismatch",
    );
    compare_list(
        assessment,
        hints.full_version_list.as_deref().map(brand_versions),
        signals.ua_full_version_list.as_deref(),
        "ua_ch_js_full_version_list_mismatch",
    );
    compare_bool(
        assessment,
        mobile,
        signals.ua_mobile,
        "ua_ch_js_mobile_mismatch",
    );
    compare_bool(
        assessment,
        structured_bool(hints.wow64.as_deref()),
        signals.ua_wow64,
        "ua_ch_js_wow64_mismatch",
    );
}

fn compare_device_profile(
    assessment: &mut UserAgentClientHintAssessment,
    platform: Option<String>,
    form_factor: Option<String>,
    object: &serde_json::Map<String, Value>,
) {
    compare_string(
        assessment,
        platform,
        object
            .get("platform")
            .and_then(Value::as_str)
            .map(normalize_platform_value),
        "ua_ch_device_platform_mismatch",
    );
    compare_string(
        assessment,
        form_factor,
        object
            .get("form_factor")
            .and_then(Value::as_str)
            .map(|value| value.trim().to_ascii_lowercase()),
        "ua_ch_device_form_factor_mismatch",
    );
}

fn normalized_token(value: Option<&str>) -> Option<String> {
    structured_string(value).map(|value| value.trim().to_ascii_lowercase())
}

fn normalized_platform(value: Option<&str>) -> Option<String> {
    structured_string(value).map(|value| normalize_platform_value(&value))
}

fn normalize_platform_value(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "macos" | "mac os" => "macos".to_string(),
        value => value.to_string(),
    }
}

fn normalized_form_factor(value: Option<&str>) -> Option<String> {
    let value = value?.to_ascii_lowercase();
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

fn legacy_platform(user_agent: Option<&str>) -> Option<&'static str> {
    let value = user_agent?.to_ascii_lowercase();
    if value.contains("android") {
        Some("android")
    } else if value.contains("iphone") || value.contains("ipad") {
        Some("ios")
    } else if value.contains("windows") {
        Some("windows")
    } else if value.contains("macintosh") || value.contains("mac os") {
        Some("macos")
    } else if value.contains("linux") {
        Some("linux")
    } else {
        None
    }
}

fn legacy_mobile(user_agent: Option<&str>) -> Option<bool> {
    let value = user_agent?.to_ascii_lowercase();
    (value.contains("mozilla/") || value.contains("chrom"))
        .then(|| value.contains("mobile") || value.contains("iphone"))
}

fn compare_string(
    assessment: &mut UserAgentClientHintAssessment,
    header: Option<String>,
    client: Option<String>,
    factor: &'static str,
) {
    if let (Some(header), Some(client)) = (header, client)
        && header != client
    {
        assessment.add(0.15, factor);
    }
}

fn compare_bool(
    assessment: &mut UserAgentClientHintAssessment,
    header: Option<bool>,
    client: Option<bool>,
    factor: &'static str,
) {
    if let (Some(header), Some(client)) = (header, client)
        && header != client
    {
        assessment.add(0.15, factor);
    }
}

fn compare_list(
    assessment: &mut UserAgentClientHintAssessment,
    header: Option<Vec<String>>,
    client: Option<&[String]>,
    factor: &'static str,
) {
    let (Some(mut header), Some(client)) = (header, client) else {
        return;
    };
    let mut client = client
        .iter()
        .map(|value| value.trim().to_ascii_lowercase())
        .collect::<Vec<_>>();
    header = header
        .into_iter()
        .map(|value| value.trim().to_ascii_lowercase())
        .collect();
    header.sort_unstable();
    client.sort_unstable();
    if header != client {
        assessment.add(0.15, factor);
    }
}
