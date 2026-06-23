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
            evidence,
        }
    }
}
