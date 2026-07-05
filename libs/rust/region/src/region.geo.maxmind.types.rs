use std::net::IpAddr;

use ipnet::{IpNet, Ipv4Net, Ipv6Net};
use serde_json::Value;
use thiserror::Error;

use crate::geo::types::{GeoLocation, GeoNetworkKind, GeoNetworkRelation};

pub const MAXMIND_GEOLITE_COUNTRY_SOURCE_CODE: &str = "maxmind_geolite_country_csv";
pub const MAXMIND_GEOLITE_CITY_SOURCE_CODE: &str = "maxmind_geolite_city_csv";
pub const MAXMIND_GEOLITE_ASN_SOURCE_CODE: &str = "maxmind_geolite_asn_csv";
pub const MAXMIND_GEOLITE_WEB_SOURCE_CODE: &str = "maxmind_geolite_city_web";

pub const MAXMIND_GEOLITE_SOURCE_CODES: [&str; 4] = [
    MAXMIND_GEOLITE_COUNTRY_SOURCE_CODE,
    MAXMIND_GEOLITE_CITY_SOURCE_CODE,
    MAXMIND_GEOLITE_ASN_SOURCE_CODE,
    MAXMIND_GEOLITE_WEB_SOURCE_CODE,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaxMindGeoLiteConfig {
    pub account_id: String,
    pub license_key: String,
    pub eula_accepted: bool,
}

impl MaxMindGeoLiteConfig {
    pub fn enabled(self) -> Result<Self, MaxMindGeoLiteConfigError> {
        if !self.eula_accepted {
            return Err(MaxMindGeoLiteConfigError::EulaNotAccepted);
        }
        if self.account_id.trim().is_empty() {
            return Err(MaxMindGeoLiteConfigError::MissingAccountId);
        }
        if self.license_key.trim().is_empty() {
            return Err(MaxMindGeoLiteConfigError::MissingLicenseKey);
        }
        Ok(Self {
            account_id: self.account_id.trim().to_string(),
            license_key: self.license_key.trim().to_string(),
            eula_accepted: true,
        })
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum MaxMindGeoLiteConfigError {
    #[error("MaxMind GeoLite EULA must be explicitly accepted")]
    EulaNotAccepted,
    #[error("MaxMind account id is required")]
    MissingAccountId,
    #[error("MaxMind license key is required")]
    MissingLicenseKey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaxMindGeoLiteRange {
    pub source_code: &'static str,
    pub edition_id: &'static str,
    pub network: IpNet,
    pub country_code: Option<String>,
    pub geoname_id: Option<String>,
    pub asn: Option<i64>,
    pub organization: Option<String>,
}

pub fn maxmind_ip_relation(
    ip: IpAddr,
    payload: &Value,
) -> Option<(GeoNetworkRelation, GeoLocation)> {
    let country_code = payload
        .pointer("/country/iso_code")
        .and_then(Value::as_str)
        .or_else(|| {
            payload
                .pointer("/registered_country/iso_code")
                .and_then(Value::as_str)
        })?;
    let location = GeoLocation::from_country_code(country_code)?;
    let network = ip_single_host_network(ip);
    let relation = GeoNetworkRelation {
        source_code: MAXMIND_GEOLITE_WEB_SOURCE_CODE.to_string(),
        registry: Some("maxmind".to_string()),
        network: Some(network.to_string()),
        start_ip: None,
        end_ip: None,
        asn: payload
            .get("traits")
            .and_then(|traits| traits.get("autonomous_system_number"))
            .and_then(Value::as_i64),
        organization: payload
            .get("traits")
            .and_then(|traits| traits.get("autonomous_system_organization"))
            .and_then(Value::as_str)
            .map(str::to_string),
        source_reference: Some("https://geolite.info/geoip/v2.1/city".to_string()),
        network_kind: Some(GeoNetworkKind::Unknown),
        risk_score: Some(35),
        risk_labels: vec!["source:maxmind_geolite".to_string()],
    };
    Some((relation, location))
}

fn ip_single_host_network(ip: IpAddr) -> IpNet {
    match ip {
        IpAddr::V4(ip) => IpNet::V4(Ipv4Net::new(ip, 32).expect("valid ipv4 host network")),
        IpAddr::V6(ip) => IpNet::V6(Ipv6Net::new(ip, 128).expect("valid ipv6 host network")),
    }
}
