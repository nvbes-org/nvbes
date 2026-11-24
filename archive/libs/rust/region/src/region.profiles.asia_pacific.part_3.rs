use crate::timezones::*;
use crate::{DataRegion, LegalJurisdiction, RegionProfile};

pub const ASIA_PACIFIC_PROFILES_PART_3: &[RegionProfile] = &[
    RegionProfile {
        country_code: "MO",
        data_region: DataRegion::Apac,
        legal_jurisdiction: LegalJurisdiction::Global,
        primary_timezone: TZ_ASIA_MACAU,
        timezones: &[TZ_ASIA_MACAU],
        sub_region: Some("China"),
        display_name: Some("Macau"),
    },
    RegionProfile {
        country_code: "NF",
        data_region: DataRegion::Apac,
        legal_jurisdiction: LegalJurisdiction::App,
        primary_timezone: TZ_PACIFIC_NORFOLK,
        timezones: &[TZ_PACIFIC_NORFOLK],
        sub_region: Some("Australia"),
        display_name: Some("Norfolk Island"),
    },
    RegionProfile {
        country_code: "CX",
        data_region: DataRegion::Apac,
        legal_jurisdiction: LegalJurisdiction::App,
        primary_timezone: TZ_ASIA_JAKARTA,
        timezones: &[TZ_ASIA_JAKARTA],
        sub_region: Some("Australia"),
        display_name: Some("Christmas Island"),
    },
    RegionProfile {
        country_code: "CC",
        data_region: DataRegion::Apac,
        legal_jurisdiction: LegalJurisdiction::App,
        primary_timezone: TZ_ASIA_JAKARTA,
        timezones: &[TZ_ASIA_JAKARTA],
        sub_region: Some("Australia"),
        display_name: Some("Cocos (Keeling) Islands"),
    },
    RegionProfile {
        country_code: "PN",
        data_region: DataRegion::Apac,
        legal_jurisdiction: LegalJurisdiction::Gdpr,
        primary_timezone: TZ_PACIFIC_PITCAIRN,
        timezones: &[TZ_PACIFIC_PITCAIRN],
        sub_region: Some("United Kingdom"),
        display_name: Some("Pitcairn Islands"),
    },
];
