use std::net::IpAddr;

use serde::{Deserialize, Serialize};

use crate::{DataRegion, LegalJurisdiction, detect_profile_from_country_code};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GeoConfidence {
    None,
    Low,
    Medium,
    High,
}

impl GeoConfidence {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GeoEvidenceSource {
    TrustedProxyHeader,
    PersonalDatabase,
    ProviderHeader,
    RemoteLookup,
    StoredProfile,
    PrivateNetwork,
    Fallback,
}

impl GeoEvidenceSource {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TrustedProxyHeader => "trusted_proxy_header",
            Self::PersonalDatabase => "personal_database",
            Self::ProviderHeader => "provider_header",
            Self::RemoteLookup => "remote_lookup",
            Self::StoredProfile => "stored_profile",
            Self::PrivateNetwork => "private_network",
            Self::Fallback => "fallback",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GeoNetworkKind {
    Unknown,
    Residential,
    Mobile,
    Datacenter,
    Vpn,
    Proxy,
    Tor,
}

impl GeoNetworkKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Residential => "residential",
            Self::Mobile => "mobile",
            Self::Datacenter => "datacenter",
            Self::Vpn => "vpn",
            Self::Proxy => "proxy",
            Self::Tor => "tor",
        }
    }

    pub fn from_label(value: &str) -> Self {
        match value {
            "residential" => Self::Residential,
            "mobile" => Self::Mobile,
            "datacenter" => Self::Datacenter,
            "vpn" => Self::Vpn,
            "proxy" => Self::Proxy,
            "tor" => Self::Tor,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GeoRiskSignal {
    Unknown,
    Residential,
    Mobile,
    Datacenter,
    Vpn,
    Proxy,
    Tor,
}

impl GeoRiskSignal {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Residential => "residential",
            Self::Mobile => "mobile",
            Self::Datacenter => "datacenter",
            Self::Vpn => "vpn",
            Self::Proxy => "proxy",
            Self::Tor => "tor",
        }
    }
}

pub fn canonical_geo_risk_label(value: &str) -> Option<&'static str> {
    let normalized = value
        .trim()
        .trim_start_matches("is_")
        .replace(['-', ' '], "_")
        .to_ascii_lowercase();
    match normalized.as_str() {
        "tor" | "tor_exit" => Some("tor"),
        "vpn" | "anonymous_vpn" | "commercial_vpn" => Some("vpn"),
        "proxy" | "socks" | "socks_proxy" | "web_proxy" | "residential_proxy" => Some("proxy"),
        "datacenter" | "data_center" | "hosting" | "hosted" | "cloud" | "cloud_provider"
        | "server" | "cdn" => Some("datacenter"),
        "mobile" | "cellular" | "wireless" | "lte" | "4g" | "5g" | "mobile_carrier" => {
            Some("mobile")
        }
        "residential" | "consumer" | "consumer_isp" | "broadband" | "fiber" | "fibre" | "cable"
        | "dsl" => Some("residential"),
        "unknown" | "unclassified" => Some("unknown"),
        _ => None,
    }
}

pub fn canonicalize_geo_risk_labels(labels: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut canonical = Vec::new();
    for label in labels {
        let label = label.trim().replace([' ', '-'], "_").to_ascii_lowercase();
        if preserves_network_label_namespace(&label) {
            push_unique_label(&mut canonical, label);
            continue;
        }
        if let Some(label) = canonical_geo_risk_label(&label) {
            push_unique_label(&mut canonical, label.to_string());
        }
    }
    canonical
}

fn preserves_network_label_namespace(label: &str) -> bool {
    matches!(
        label.split_once(':').map(|(namespace, _)| namespace),
        Some("source" | "provider" | "asn" | "category" | "mismatch" | "velocity" | "payment")
    )
}

pub fn push_unique_label(labels: &mut Vec<String>, label: impl Into<String>) {
    let label = label.into();
    if !labels.iter().any(|item| item == &label) {
        labels.push(label);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeoNetworkRelation {
    pub source_code: String,
    pub registry: Option<String>,
    pub network: Option<String>,
    pub start_ip: Option<IpAddr>,
    pub end_ip: Option<IpAddr>,
    pub asn: Option<i64>,
    pub organization: Option<String>,
    pub source_reference: Option<String>,
    pub network_kind: Option<GeoNetworkKind>,
    pub risk_score: Option<u8>,
    pub risk_labels: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeoLocation {
    pub country_code: String,
    pub data_region: DataRegion,
    pub legal_jurisdiction: LegalJurisdiction,
}

impl GeoLocation {
    pub fn from_country_code(country_code: &str) -> Option<Self> {
        let profile = detect_profile_from_country_code(country_code)?;
        Some(Self {
            country_code: profile.country_code.to_string(),
            data_region: profile.data_region,
            legal_jurisdiction: profile.legal_jurisdiction,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeoEvidence {
    pub source: GeoEvidenceSource,
    pub country_code: Option<String>,
    pub confidence: GeoConfidence,
    pub accepted: bool,
    pub reason: String,
    pub relation: Option<GeoNetworkRelation>,
}

impl GeoEvidence {
    pub fn accepted(
        source: GeoEvidenceSource,
        location: &GeoLocation,
        confidence: GeoConfidence,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            source,
            country_code: Some(location.country_code.clone()),
            confidence,
            accepted: true,
            reason: reason.into(),
            relation: None,
        }
    }

    pub fn accepted_with_relation(
        source: GeoEvidenceSource,
        location: &GeoLocation,
        confidence: GeoConfidence,
        reason: impl Into<String>,
        relation: GeoNetworkRelation,
    ) -> Self {
        Self {
            source,
            country_code: Some(location.country_code.clone()),
            confidence,
            accepted: true,
            reason: reason.into(),
            relation: Some(relation),
        }
    }

    pub fn rejected(
        source: GeoEvidenceSource,
        country_code: Option<&str>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            source,
            country_code: country_code.map(str::to_ascii_uppercase),
            confidence: GeoConfidence::None,
            accepted: false,
            reason: reason.into(),
            relation: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeoResolution {
    pub location: Option<GeoLocation>,
    pub confidence: GeoConfidence,
    pub source: GeoEvidenceSource,
    pub ip: Option<IpAddr>,
    pub private_network: bool,
    pub network_kind: GeoNetworkKind,
    pub risk_score: u8,
    pub risk_labels: Vec<String>,
    pub evidence: Vec<GeoEvidence>,
}

impl GeoResolution {
    pub fn unresolved(
        ip: Option<IpAddr>,
        private_network: bool,
        evidence: Vec<GeoEvidence>,
    ) -> Self {
        Self {
            location: None,
            confidence: GeoConfidence::None,
            source: GeoEvidenceSource::Fallback,
            ip,
            private_network,
            network_kind: GeoNetworkKind::Unknown,
            risk_score: if private_network { 0 } else { 50 },
            risk_labels: if private_network {
                vec!["private_network".to_string()]
            } else {
                vec!["unknown".to_string()]
            },
            evidence,
        }
    }
}

#[cfg(test)]
#[path = "region.geo.types.tests.rs"]
mod tests;
