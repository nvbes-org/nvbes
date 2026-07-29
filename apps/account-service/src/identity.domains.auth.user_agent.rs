use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[path = "identity.domains.auth.user_agent.browser.rs"]
mod browser;
#[path = "identity.domains.auth.user_agent.device.rs"]
mod device;
#[path = "identity.domains.auth.user_agent.risk.rs"]
pub mod risk;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct UserAgentInfo {
    pub browser: Option<String>,
    pub browser_version: Option<f64>,
    pub os: Option<String>,
    pub os_version: Option<String>,
    pub device: Option<String>,
    pub device_type: String,
}

pub fn parse(user_agent: Option<&str>, browser_brands: Option<&str>) -> Option<UserAgentInfo> {
    let user_agent = user_agent?.trim();
    if user_agent.is_empty() {
        return None;
    }

    let browser = browser::detect_browser(user_agent, browser_brands);
    let (os, os_version) = device::detect_os(user_agent);
    let device = device::detect_device(user_agent);
    let device_type = device::detect_device_type(device, browser, os);

    Some(UserAgentInfo {
        browser: browser.map(str::to_string),
        browser_version: browser.and_then(|name| browser::detect_browser_version(user_agent, name)),
        os: os.map(str::to_string),
        os_version,
        device: device.map(str::to_string),
        device_type: device_type.to_string(),
    })
}

pub fn stable_family(user_agent: Option<&str>, browser_brands: Option<&str>) -> Option<String> {
    let info = parse(user_agent, browser_brands)?;
    Some(format!(
        "{}:{}:{}",
        info.browser.as_deref().unwrap_or("unknown"),
        info.os.as_deref().unwrap_or("unknown"),
        info.device_type
    ))
}

#[cfg(test)]
#[path = "identity.domains.auth.user_agent.tests.rs"]
mod tests;
