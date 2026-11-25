use serde::{Deserialize, Serialize};

use super::types::{GeoLocation, GeoNetworkKind};
use super::v2fly_dat::V2flyGeoIpDatEntry;

pub const LOYALSOLDIER_GEOIP_SOURCE_CODE: &str = "loyalsoldier_geoip";
pub const LOYALSOLDIER_GEOIP_DAT_URL: &str =
    "https://raw.githubusercontent.com/Loyalsoldier/geoip/release/geoip.dat";
pub const LOYALSOLDIER_GEOIP_SHA256_URL: &str =
    "https://raw.githubusercontent.com/Loyalsoldier/geoip/release/geoip.dat.sha256sum";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoyalsoldierGeoIpImportReport {
    pub downloaded: bool,
    pub imported_ranges: u64,
    pub expired_ranges: u64,
    pub country_ranges: u64,
    pub category_ranges: u64,
}

#[derive(Debug)]
pub struct LoyalsoldierMappedRange {
    pub key: String,
    pub code: String,
    pub country_code: Option<String>,
    pub network: String,
    pub kind: String,
    pub score: i16,
    pub labels: Vec<String>,
}

pub fn map_loyalsoldier_range(range: &V2flyGeoIpDatEntry) -> LoyalsoldierMappedRange {
    let code = range.code.to_ascii_lowercase();
    let country_code = loyalsoldier_country_code(&code);
    let (kind, score, mut labels) = category_reputation(&code, country_code.is_some());
    labels.push("source:loyalsoldier_geoip".to_string());

    LoyalsoldierMappedRange {
        key: format!("{}:{}", code, range.network),
        code,
        country_code,
        network: range.network.to_string(),
        kind: kind.as_str().to_string(),
        score,
        labels,
    }
}

pub fn loyalsoldier_country_code(code: &str) -> Option<String> {
    let country_code = code.to_ascii_uppercase();
    GeoLocation::from_country_code(&country_code).map(|location| location.country_code)
}

pub fn loyalsoldier_is_country(code: &str) -> bool {
    loyalsoldier_country_code(code).is_some()
}

fn category_reputation(code: &str, country: bool) -> (GeoNetworkKind, i16, Vec<String>) {
    match code {
        _ if country => (
            GeoNetworkKind::Unknown,
            35,
            vec!["country_geoip".to_string()],
        ),
        "tor" => (GeoNetworkKind::Tor, 95, vec!["tor".to_string()]),
        "private" => (
            GeoNetworkKind::Unknown,
            0,
            vec!["private_network".to_string()],
        ),
        "cloudflare" | "cloudfront" | "fastly" | "google" => (
            GeoNetworkKind::Datacenter,
            65,
            vec!["datacenter".to_string(), format!("provider:{code}")],
        ),
        "facebook" | "netflix" | "telegram" | "twitter" => (
            GeoNetworkKind::Datacenter,
            60,
            vec!["datacenter".to_string(), format!("provider:{code}")],
        ),
        _ => (
            GeoNetworkKind::Unknown,
            45,
            vec![format!("category:{code}")],
        ),
    }
}

#[cfg(test)]
#[path = "region.geo.loyalsoldier.types.tests.rs"]
mod tests;
