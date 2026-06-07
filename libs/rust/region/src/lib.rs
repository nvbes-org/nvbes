use serde::{Deserialize, Serialize};
use thiserror::Error;

#[path = "region.timezones.rs"]
pub mod timezones;

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
mod tests {
    use super::{
        DataRegion, country_code_to_data_region, detect_profile_from_country_code,
        supported_profiles,
    };
    use crate::timezones::{TZ_ATLANTIC_CANARY, TZ_EUROPE_MADRID};

    #[test]
    fn detects_supported_profile() {
        assert!(detect_profile_from_country_code("FR").is_some());
        assert!(detect_profile_from_country_code("DE").is_some());
        assert!(detect_profile_from_country_code("CH").is_some());
        assert!(detect_profile_from_country_code("XX").is_none());
    }

    #[test]
    fn maps_data_region_from_country() {
        assert_eq!(country_code_to_data_region("CH"), Some(DataRegion::Ch));
        assert_eq!(country_code_to_data_region("FR"), Some(DataRegion::Eu));
        assert_eq!(country_code_to_data_region("XX"), None);
    }

    #[test]
    fn assigns_correct_timezone() {
        let profile = detect_profile_from_country_code("AT").unwrap();
        assert_eq!(profile.primary_timezone.as_str(), "Europe/Vienna");
        let profile = detect_profile_from_country_code("DE").unwrap();
        assert_eq!(profile.primary_timezone.as_str(), "Europe/Berlin");
        let profile = detect_profile_from_country_code("FR").unwrap();
        assert_eq!(profile.primary_timezone.as_str(), "Europe/Paris");
        let profile = detect_profile_from_country_code("IS").unwrap();
        assert_eq!(profile.primary_timezone.as_str(), "Atlantic/Reykjavik");
        let profile = detect_profile_from_country_code("CH").unwrap();
        assert_eq!(profile.primary_timezone.as_str(), "Europe/Zurich");
    }

    #[test]
    fn cyprus_uses_europe_nicosia() {
        let profile = detect_profile_from_country_code("CY").unwrap();
        assert_eq!(profile.primary_timezone.as_str(), "Europe/Nicosia");
    }

    #[test]
    fn liechtenstein_uses_zurich() {
        let profile = detect_profile_from_country_code("LI").unwrap();
        assert_eq!(profile.primary_timezone.as_str(), "Europe/Zurich");
    }

    #[test]
    fn switzerland_has_nfdap_jurisdiction() {
        let profile = detect_profile_from_country_code("CH").unwrap();
        assert_eq!(profile.legal_jurisdiction.as_str(), "nfdap");
    }

    #[test]
    fn spain_has_canary_timezone() {
        let profile = detect_profile_from_country_code("ES").unwrap();
        assert!(profile.timezones.len() == 2);
        assert!(profile.timezones.contains(&TZ_ATLANTIC_CANARY));
        assert!(profile.timezones.contains(&TZ_EUROPE_MADRID));
    }

    #[test]
    fn french_overseas_have_separate_profiles() {
        assert!(detect_profile_from_country_code("GF").is_some());
        assert!(detect_profile_from_country_code("GP").is_some());
        assert!(detect_profile_from_country_code("MQ").is_some());
        assert!(detect_profile_from_country_code("RE").is_some());
    }

    #[test]
    fn all_profile_timezones_are_valid() {
        for profile in supported_profiles() {
            let primary = profile.primary_timezone.as_str();
            assert!(
                !primary.is_empty(),
                "empty timezone for {}",
                profile.country_code
            );
            for tz in profile.timezones {
                assert!(
                    !tz.as_str().is_empty(),
                    "empty sub-timezone for {}",
                    profile.country_code
                );
            }
            assert!(
                profile.timezones.contains(&profile.primary_timezone),
                "primary timezone not in timezones for {}",
                profile.country_code,
            );
        }
    }
}
