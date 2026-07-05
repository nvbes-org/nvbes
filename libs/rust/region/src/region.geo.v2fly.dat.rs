use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use ipnet::{IpNet, Ipv4Net, Ipv6Net};
use prost::Message;
use thiserror::Error;

use crate::geo::types::GeoLocation;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum V2flyGeoIpDatError {
    #[error("v2fly geoip.dat protobuf decode failed")]
    Decode,
    #[error("v2fly geoip.dat contains invalid cidr bytes")]
    InvalidCidr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct V2flyGeoIpRange {
    pub country_code: String,
    pub network: IpNet,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct V2flyGeoIpDatEntry {
    pub code: String,
    pub network: IpNet,
}

#[derive(Clone, PartialEq, Message)]
struct GeoIpList {
    #[prost(message, repeated, tag = "1")]
    entry: Vec<GeoIp>,
}

#[derive(Clone, PartialEq, Message)]
struct GeoIp {
    #[prost(string, tag = "1")]
    country_code: String,
    #[prost(message, repeated, tag = "2")]
    cidr: Vec<Cidr>,
}

#[derive(Clone, PartialEq, Message)]
struct Cidr {
    #[prost(bytes, tag = "1")]
    ip: Vec<u8>,
    #[prost(uint32, tag = "2")]
    prefix: u32,
}

pub fn parse_v2fly_geoip_dat(bytes: &[u8]) -> Result<Vec<V2flyGeoIpRange>, V2flyGeoIpDatError> {
    let mut ranges = Vec::new();

    for entry in parse_v2fly_geoip_dat_entries(bytes)? {
        let country_code = entry.code.to_ascii_uppercase();
        if GeoLocation::from_country_code(&country_code).is_none() {
            continue;
        }

        ranges.push(V2flyGeoIpRange {
            country_code,
            network: entry.network,
        });
    }

    Ok(ranges)
}

pub fn parse_v2fly_geoip_dat_entries(
    bytes: &[u8],
) -> Result<Vec<V2flyGeoIpDatEntry>, V2flyGeoIpDatError> {
    let list = GeoIpList::decode(bytes).map_err(|_| V2flyGeoIpDatError::Decode)?;
    let mut ranges = Vec::new();

    for entry in list.entry {
        let code = entry.country_code.to_ascii_lowercase();
        for cidr in entry.cidr {
            ranges.push(V2flyGeoIpDatEntry {
                code: code.clone(),
                network: cidr_to_ipnet(cidr)?,
            });
        }
    }

    Ok(ranges)
}

fn cidr_to_ipnet(cidr: Cidr) -> Result<IpNet, V2flyGeoIpDatError> {
    match cidr.ip.len() {
        4 => {
            let ip = IpAddr::V4(Ipv4Addr::new(
                cidr.ip[0], cidr.ip[1], cidr.ip[2], cidr.ip[3],
            ));
            Ipv4Net::new(
                match ip {
                    IpAddr::V4(ip) => ip,
                    IpAddr::V6(_) => unreachable!("constructed ipv4 address"),
                },
                u8::try_from(cidr.prefix).map_err(|_| V2flyGeoIpDatError::InvalidCidr)?,
            )
            .map(IpNet::V4)
            .map_err(|_| V2flyGeoIpDatError::InvalidCidr)
        }
        16 => {
            let octets: [u8; 16] = cidr
                .ip
                .try_into()
                .map_err(|_| V2flyGeoIpDatError::InvalidCidr)?;
            Ipv6Net::new(
                Ipv6Addr::from(octets),
                u8::try_from(cidr.prefix).map_err(|_| V2flyGeoIpDatError::InvalidCidr)?,
            )
            .map(IpNet::V6)
            .map_err(|_| V2flyGeoIpDatError::InvalidCidr)
        }
        _ => Err(V2flyGeoIpDatError::InvalidCidr),
    }
}

#[cfg(test)]
mod tests {
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
}
