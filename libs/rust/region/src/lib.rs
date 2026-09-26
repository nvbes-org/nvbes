use serde::{Deserialize, Serialize};
use thiserror::Error;

#[path = "region.timezones.rs"]
pub mod timezones;

#[path = "region.geo.rs"]
pub mod geo;

#[path = "region.profiles.mod.rs"]
mod profiles;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DataRegion {
    Eu,
    Us,
    Ch,
    Apac,
    Uk,
    Latam,
    MeAfrica,
}

impl DataRegion {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Eu => "eu",
            Self::Us => "us",
            Self::Ch => "ch",
            Self::Apac => "apac",
            Self::Uk => "uk",
            Self::Latam => "latam",
            Self::MeAfrica => "me_africa",
        }
    }

    pub const fn is_european_exclusive(self) -> bool {
        matches!(self, Self::Eu | Self::Uk | Self::Ch)
    }

    pub const fn is_global_multicloud(self) -> bool {
        !self.is_european_exclusive()
    }

    pub const fn hosting_strategy(self) -> &'static str {
        if self.is_european_exclusive() {
            "exclusive_eu_sovereign"
        } else {
            "global_multicloud"
        }
    }
}

impl core::str::FromStr for DataRegion {
    type Err = RegionParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "eu" => Ok(Self::Eu),
            "us" => Ok(Self::Us),
            "ch" => Ok(Self::Ch),
            "apac" => Ok(Self::Apac),
            "uk" => Ok(Self::Uk),
            "latam" => Ok(Self::Latam),
            "me_africa" => Ok(Self::MeAfrica),
            _ => Err(RegionParseError::UnsupportedDataRegion),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LegalJurisdiction {
    Gdpr,
    Ccpa,
    Nfdap,
    UkGdpr,
    Lgpd,
    Pipeda,
    Popia,
    App,
    Pdpa,
    Pipl,
    Global,
    Ndpa,
    Appi,
    Pipa,
    Nzpa,
    Cndp,
    Inpdp,
    Edpl,
    Kdpa,
    Apdp,
}

impl LegalJurisdiction {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Gdpr => "gdpr",
            Self::Ccpa => "ccpa",
            Self::Nfdap => "nfdap",
            Self::UkGdpr => "uk_gdpr",
            Self::Lgpd => "lgpd",
            Self::Pipeda => "pipeda",
            Self::Popia => "popia",
            Self::App => "app",
            Self::Pdpa => "pdpa",
            Self::Pipl => "pipl",
            Self::Global => "global",
            Self::Ndpa => "ndpa",
            Self::Appi => "appi",
            Self::Pipa => "pipa",
            Self::Nzpa => "nzpa",
            Self::Cndp => "cndp",
            Self::Inpdp => "inpdp",
            Self::Edpl => "edpl",
            Self::Kdpa => "kdpa",
            Self::Apdp => "apdp",
        }
    }
}

impl core::str::FromStr for LegalJurisdiction {
    type Err = RegionParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "gdpr" => Ok(Self::Gdpr),
            "ccpa" => Ok(Self::Ccpa),
            "nfdap" => Ok(Self::Nfdap),
            "uk_gdpr" => Ok(Self::UkGdpr),
            "lgpd" => Ok(Self::Lgpd),
            "pipeda" => Ok(Self::Pipeda),
            "popia" => Ok(Self::Popia),
            "app" => Ok(Self::App),
            "pdpa" => Ok(Self::Pdpa),
            "pipl" => Ok(Self::Pipl),
            "global" => Ok(Self::Global),
            "ndpa" => Ok(Self::Ndpa),
            "appi" => Ok(Self::Appi),
            "pipa" => Ok(Self::Pipa),
            "nzpa" => Ok(Self::Nzpa),
            "cndp" => Ok(Self::Cndp),
            "inpdp" => Ok(Self::Inpdp),
            "edpl" => Ok(Self::Edpl),
            "kdpa" => Ok(Self::Kdpa),
            "apdp" => Ok(Self::Apdp),
            _ => Err(RegionParseError::UnsupportedLegalJurisdiction),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Timezone(pub(crate) &'static str);

impl Timezone {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

impl Serialize for Timezone {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl core::fmt::Display for Timezone {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct RegionProfile {
    pub country_code: &'static str,
    pub data_region: DataRegion,
    pub legal_jurisdiction: LegalJurisdiction,
    pub primary_timezone: Timezone,
    pub timezones: &'static [Timezone],
    pub sub_region: Option<&'static str>,
    pub display_name: Option<&'static str>,
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum RegionParseError {
    #[error("unsupported data region")]
    UnsupportedDataRegion,
    #[error("unsupported legal jurisdiction")]
    UnsupportedLegalJurisdiction,
}

pub fn detect_profile_from_country_code(country_code: &str) -> Option<RegionProfile> {
    let country_code = country_code.trim().to_ascii_uppercase();
    supported_profiles()
        .iter()
        .find(|profile| profile.country_code == country_code)
        .copied()
}

pub fn profile_from_data_region(data_region: DataRegion) -> RegionProfile {
    supported_profiles()
        .iter()
        .find(|profile| profile.data_region == data_region)
        .copied()
        .expect("supported data region must have a default profile")
}

pub fn country_code_to_data_region(country_code: &str) -> Option<DataRegion> {
    detect_profile_from_country_code(country_code).map(|profile| profile.data_region)
}

pub const ALLOWED_COUNTRY_CODES: &[&str] = &[
    // Europe (EU, EEE, UK, Non-EU Europe)
    "AD", "AL", "AT", "BA", "BE", "BG", "CH", "CY", "CZ", "DE", "DK", "EE", "ES", "FI", "FO", "FR",
    "GB", "GG", "GI", "GR", "HR", "HU", "IE", "IM", "IS", "IT", "JE", "LI", "LT", "LU", "LV", "MC",
    "MD", "ME", "MK", "MT", "NL", "NO", "PL", "PT", "RO", "RS", "SE", "SI", "SK", "SM", "VA",
    // North America
    "CA", "US", // Latin America & Caribbean
    "AG", "AI", "AR", "AW", "BB", "BL", "BM", "BO", "BR", "BS", "BZ", "CL", "CO", "CR", "CU", "CW",
    "DM", "DO", "EC", "FK", "GD", "GF", "GP", "GT", "GY", "HN", "HT", "JM", "KN", "KY", "LC", "MF",
    "MQ", "MX", "NI", "PA", "PE", "PM", "PR", "PY", "SR", "SV", "SX", "TC", "TT", "UY", "VC", "VE",
    "VG", "VI", // APAC & SE Asia
    "AU", "BN", "ID", "JP", "KR", "MY", "NZ", "PH", "SG", "TH", "VN", // Middle East
    "AE", "BH", "IL", "JO", "KW", "LB", "OM", "QA", "SA",
    // Africa (North, West/Central, East/Southern, NG, ET, DZ)
    "AO", "BF", "BI", "BJ", "BW", "CD", "CF", "CG", "CI", "CM", "CV", "DJ", "DZ", "EG", "ER", "ET",
    "GA", "GH", "GM", "GN", "GQ", "GW", "KE", "KM", "LR", "LS", "LY", "MA", "MG", "ML", "MR", "MU",
    "MW", "MZ", "NA", "NE", "NG", "RW", "SC", "SD", "SL", "SN", "SO", "SS", "ST", "SZ", "TD", "TG",
    "TN", "TZ", "UG", "ZA", "ZM", "ZW",
];

pub fn is_country_allowed(country_code: &str) -> bool {
    let code = country_code.trim().to_ascii_uppercase();
    ALLOWED_COUNTRY_CODES.iter().any(|&c| c == code)
}

pub fn supported_data_regions() -> &'static [DataRegion] {
    &[
        DataRegion::Eu,
        DataRegion::Us,
        DataRegion::Ch,
        DataRegion::Apac,
        DataRegion::Uk,
        DataRegion::Latam,
        DataRegion::MeAfrica,
    ]
}

pub fn supported_profiles() -> &'static [RegionProfile] {
    &profiles::ALL_PROFILES
}

#[cfg(test)]
#[path = "region.lib.tests.rs"]
mod tests;
