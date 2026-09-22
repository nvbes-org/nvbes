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
#[path = "region.geo.v2fly.dat.tests.rs"]
mod tests;
