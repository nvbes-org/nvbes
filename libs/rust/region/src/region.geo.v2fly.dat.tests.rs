use prost::Message;

use super::{Cidr, GeoIp, GeoIpList, parse_v2fly_geoip_dat, parse_v2fly_geoip_dat_entries};

#[test]
fn parses_country_cidrs_and_skips_non_country_entries() {
    let dat = GeoIpList {
        entry: vec![
            GeoIp {
                country_code: "fr".to_string(),
                cidr: vec![Cidr {
                    ip: vec![203, 0, 113, 0],
                    prefix: 24,
                }],
            },
            GeoIp {
                country_code: "private".to_string(),
                cidr: vec![Cidr {
                    ip: vec![10, 0, 0, 0],
                    prefix: 8,
                }],
            },
        ],
    }
    .encode_to_vec();

    let ranges = parse_v2fly_geoip_dat(&dat).unwrap();

    assert_eq!(ranges.len(), 1);
    assert_eq!(ranges[0].country_code, "FR");
    assert_eq!(ranges[0].network.to_string(), "203.0.113.0/24");
}

#[test]
fn parses_all_categories_for_enriched_sources() {
    let dat = GeoIpList {
        entry: vec![GeoIp {
            country_code: "tor".to_string(),
            cidr: vec![Cidr {
                ip: vec![198, 51, 100, 0],
                prefix: 24,
            }],
        }],
    }
    .encode_to_vec();

    let ranges = parse_v2fly_geoip_dat_entries(&dat).unwrap();

    assert_eq!(ranges.len(), 1);
    assert_eq!(ranges[0].code, "tor");
    assert_eq!(ranges[0].network.to_string(), "198.51.100.0/24");
}

#[test]
fn parses_ipv6_cidrs_from_local_fixture() {
    let mut ip = vec![0_u8; 16];
    ip[0] = 0x20;
    ip[1] = 0x01;
    ip[2] = 0x0d;
    ip[3] = 0xb8;
    let dat = GeoIpList {
        entry: vec![GeoIp {
            country_code: "US".to_string(),
            cidr: vec![Cidr { ip, prefix: 32 }],
        }],
    }
    .encode_to_vec();

    let ranges = parse_v2fly_geoip_dat(&dat).unwrap();
    assert_eq!(ranges.len(), 1);
    assert_eq!(ranges[0].country_code, "US");
    assert_eq!(ranges[0].network.to_string(), "2001:db8::/32");
}

#[test]
fn rejects_invalid_cidr_bytes() {
    let dat = GeoIpList {
        entry: vec![GeoIp {
            country_code: "US".to_string(),
            cidr: vec![Cidr {
                ip: vec![127],
                prefix: 8,
            }],
        }],
    }
    .encode_to_vec();

    assert!(parse_v2fly_geoip_dat(&dat).is_err());
}

#[test]
fn rejects_oversized_prefix() {
    let dat = GeoIpList {
        entry: vec![GeoIp {
            country_code: "FR".to_string(),
            cidr: vec![Cidr {
                ip: vec![203, 0, 113, 0],
                prefix: 300,
            }],
        }],
    }
    .encode_to_vec();
    assert!(parse_v2fly_geoip_dat(&dat).is_err());
}

#[test]
fn rejects_garbage_protobuf() {
    assert!(parse_v2fly_geoip_dat(&[0xff, 0x00, 0x01]).is_err());
}
