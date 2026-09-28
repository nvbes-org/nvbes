use std::str::FromStr;

use super::{
    DataRegion, LegalJurisdiction, RegionParseError, country_code_to_data_region,
    detect_profile_from_country_code, is_country_allowed, profile_from_data_region,
    supported_data_regions, supported_profiles,
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
fn data_region_from_str_and_hosting_strategy() {
    for (raw, region) in [
        ("eu", DataRegion::Eu),
        ("us", DataRegion::Us),
        ("ch", DataRegion::Ch),
        ("apac", DataRegion::Apac),
        ("uk", DataRegion::Uk),
        ("latam", DataRegion::Latam),
        ("me_africa", DataRegion::MeAfrica),
    ] {
        assert_eq!(DataRegion::from_str(raw), Ok(region));
        assert_eq!(DataRegion::from_str(&raw.to_ascii_uppercase()), Ok(region));
        assert_eq!(region.as_str(), raw);
    }
    assert_eq!(
        DataRegion::from_str("invalid"),
        Err(RegionParseError::UnsupportedDataRegion)
    );
    assert!(DataRegion::Eu.is_european_exclusive());
    assert!(DataRegion::Uk.is_european_exclusive());
    assert!(DataRegion::Ch.is_european_exclusive());
    assert!(!DataRegion::Us.is_european_exclusive());
    assert!(!DataRegion::Apac.is_european_exclusive());
    assert!(!DataRegion::Latam.is_european_exclusive());
    assert!(!DataRegion::MeAfrica.is_european_exclusive());
    assert!(!DataRegion::Eu.is_global_multicloud());
    assert!(!DataRegion::Uk.is_global_multicloud());
    assert!(!DataRegion::Ch.is_global_multicloud());
    assert!(DataRegion::Us.is_global_multicloud());
    assert!(DataRegion::Apac.is_global_multicloud());
    assert_eq!(DataRegion::Us.hosting_strategy(), "global_multicloud");
    assert_eq!(DataRegion::Ch.hosting_strategy(), "exclusive_eu_sovereign");
}

#[test]
fn legal_jurisdiction_from_str_covers_all_codes() {
    for (raw, jurisdiction) in [
        ("gdpr", LegalJurisdiction::Gdpr),
        ("ccpa", LegalJurisdiction::Ccpa),
        ("nfdap", LegalJurisdiction::Nfdap),
        ("uk_gdpr", LegalJurisdiction::UkGdpr),
        ("lgpd", LegalJurisdiction::Lgpd),
        ("pipeda", LegalJurisdiction::Pipeda),
        ("popia", LegalJurisdiction::Popia),
        ("app", LegalJurisdiction::App),
        ("pdpa", LegalJurisdiction::Pdpa),
        ("pipl", LegalJurisdiction::Pipl),
        ("global", LegalJurisdiction::Global),
        ("ndpa", LegalJurisdiction::Ndpa),
        ("appi", LegalJurisdiction::Appi),
        ("pipa", LegalJurisdiction::Pipa),
        ("nzpa", LegalJurisdiction::Nzpa),
        ("cndp", LegalJurisdiction::Cndp),
        ("inpdp", LegalJurisdiction::Inpdp),
        ("edpl", LegalJurisdiction::Edpl),
        ("kdpa", LegalJurisdiction::Kdpa),
        ("apdp", LegalJurisdiction::Apdp),
    ] {
        assert_eq!(LegalJurisdiction::from_str(raw), Ok(jurisdiction));
        assert_eq!(
            LegalJurisdiction::from_str(&raw.to_ascii_uppercase()),
            Ok(jurisdiction)
        );
        assert_eq!(jurisdiction.as_str(), raw);
    }
    assert_eq!(
        LegalJurisdiction::from_str("unknown"),
        Err(RegionParseError::UnsupportedLegalJurisdiction)
    );
}

#[test]
fn timezone_display_writes_iana_name() {
    assert_eq!(format!("{}", TZ_EUROPE_MADRID), "Europe/Madrid");
    assert_eq!(TZ_EUROPE_MADRID.to_string(), "Europe/Madrid");
}

#[test]
fn is_country_allowed_normalizes_input() {
    assert!(is_country_allowed(" fr "));
    assert!(!is_country_allowed("ZZ"));
}

#[test]
fn profile_from_data_region_returns_default_for_each_region() {
    for region in supported_data_regions() {
        let profile = profile_from_data_region(*region);
        assert_eq!(profile.data_region, *region);
        assert!(profile.primary_timezone.as_str().contains('/'));
    }
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
