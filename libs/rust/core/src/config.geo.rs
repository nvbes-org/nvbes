use super::env::{optional_env, parse_csv};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpIntelligenceEnv {
    pub provider_specs: Vec<String>,
    pub timeout_secs: u64,
    pub cache_ttl_hours: i64,
}

pub fn ip_intelligence_env() -> IpIntelligenceEnv {
    IpIntelligenceEnv {
        provider_specs: optional_env("NVBES_IP_INTELLIGENCE_PROVIDERS")
            .map(|value| parse_csv(&value))
            .unwrap_or_default(),
        timeout_secs: std::env::var("NVBES_IP_INTELLIGENCE_TIMEOUT_SECS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(2),
        cache_ttl_hours: std::env::var("NVBES_IP_INTELLIGENCE_CACHE_TTL_HOURS")
            .ok()
            .and_then(|value| value.parse::<i64>().ok())
            .unwrap_or(24 * 7),
    }
}
