use proptest::prelude::*;
use std::net::IpAddr;

use super::{
    ip::{is_private_or_special_ip, parse_ip},
    rdap::parse_asn,
    types::{GeoLocation, canonical_geo_risk_label, canonicalize_geo_risk_labels},
};

proptest! {
    #[test]
    fn parse_ip_never_panics_on_arbitrary_input(s in ".*") {
        let _ = parse_ip(&s);
    }

    #[test]
    fn parse_ip_accepts_valid_ipv4(
        a in 0u8..=255,
        b in 0u8..=255,
        c in 0u8..=255,
        d in 0u8..=255
    ) {
        let ip_str = format!("{a}.{b}.{c}.{d}");
        let parsed = parse_ip(&ip_str);
        prop_assert!(parsed.is_some());
        prop_assert_eq!(parsed.unwrap().to_string(), ip_str);
    }

    #[test]
    fn is_private_or_special_ip_classifies_standard_ranges(
        octet in 0u8..=255,
        host in 1u8..=254
    ) {
        // 10.0.0.0/8 is private
        let ip_10: IpAddr = format!("10.{octet}.0.{host}").parse().unwrap();
        prop_assert!(is_private_or_special_ip(ip_10));

        // 192.168.0.0/16 is private
        let ip_192: IpAddr = format!("192.168.{octet}.{host}").parse().unwrap();
        prop_assert!(is_private_or_special_ip(ip_192));

        // 127.0.0.0/8 is loopback
        let ip_loopback: IpAddr = format!("127.0.0.{host}").parse().unwrap();
        prop_assert!(is_private_or_special_ip(ip_loopback));
    }

    #[test]
    fn parse_asn_never_panics_on_arbitrary_input(s in ".*") {
        let _ = parse_asn(s);
    }

    #[test]
    fn parse_asn_roundtrips_valid_numbers(
        num in 1i64..4_000_000_000i64
    ) {
        let input = format!("AS{num}");
        prop_assert_eq!(parse_asn(input), Some(num));
    }

    #[test]
    fn parse_asn_rejects_non_as_handles(
        prefix in "[A-Z]{1,4}",
        handle in "[a-z0-9\\-]{4,16}"
    ) {
        prop_assume!(prefix != "AS");
        let input = format!("{prefix}-{handle}");
        prop_assert_eq!(parse_asn(input), None);
    }

    #[test]
    fn canonicalize_geo_risk_labels_never_panics(
        labels in prop::collection::vec(".*", 0..10)
    ) {
        let canonical = canonicalize_geo_risk_labels(labels);
        // Invariant: no duplicates
        let mut deduplicated = canonical.clone();
        deduplicated.sort();
        deduplicated.dedup();
        prop_assert_eq!(canonical.len(), deduplicated.len());
    }

    #[test]
    fn canonical_geo_risk_label_normalizes_known_aliases(
        label in prop::sample::select(&[
            ("tor_exit", "tor"),
            ("commercial_vpn", "vpn"),
            ("anonymous_vpn", "vpn"),
            ("web_proxy", "proxy"),
            ("cloud_provider", "datacenter"),
            ("consumer_isp", "residential"),
            ("cellular", "mobile")
        ])
    ) {
        let (alias, expected) = label;
        prop_assert_eq!(canonical_geo_risk_label(alias), Some(expected));
    }

    #[test]
    fn geo_location_from_country_code_never_panics(code in ".*") {
        let _ = GeoLocation::from_country_code(&code);
    }

    #[test]
    fn geo_location_from_country_code_resolves_known_iso(
        code in prop::sample::select(&["FR", "DE", "US", "GB", "CH", "JP", "BR", "ZA", "AU", "SG"])
    ) {
        let loc = GeoLocation::from_country_code(code);
        prop_assert!(loc.is_some());
        prop_assert_eq!(loc.unwrap().country_code, code);
    }
}
