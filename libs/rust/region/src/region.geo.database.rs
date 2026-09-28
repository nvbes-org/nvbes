use std::net::IpAddr;

use ipnet::IpNet;
use thiserror::Error;

use crate::geo::types::{GeoLocation, GeoNetworkKind, GeoNetworkRelation};

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PersonalGeoDatabaseError {
    #[error("unsupported country code for personal geo range")]
    UnsupportedCountryCode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersonalGeoRange {
    network: IpNet,
    location: GeoLocation,
    network_kind: GeoNetworkKind,
    risk_score: u8,
    risk_labels: Vec<String>,
}

impl PersonalGeoRange {
    pub fn new(network: IpNet, country_code: &str) -> Result<Self, PersonalGeoDatabaseError> {
        Self::with_reputation(
            network,
            country_code,
            GeoNetworkKind::Residential,
            15,
            vec!["personal_database".to_string(), "residential".to_string()],
        )
    }

    pub fn with_reputation(
        network: IpNet,
        country_code: &str,
        network_kind: GeoNetworkKind,
        risk_score: u8,
        risk_labels: Vec<String>,
    ) -> Result<Self, PersonalGeoDatabaseError> {
        let location = GeoLocation::from_country_code(country_code)
            .ok_or(PersonalGeoDatabaseError::UnsupportedCountryCode)?;
        Ok(Self {
            network,
            location,
            network_kind,
            risk_score,
            risk_labels,
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PersonalGeoDatabase {
    ranges: Vec<PersonalGeoRange>,
}

impl PersonalGeoDatabase {
    pub fn new(ranges: Vec<PersonalGeoRange>) -> Self {
        Self { ranges }
    }

    pub fn lookup(&self, ip: IpAddr) -> Option<GeoLocation> {
        self.lookup_relation(ip).map(|(_, location)| location)
    }

    pub fn lookup_relation(&self, ip: IpAddr) -> Option<(GeoNetworkRelation, GeoLocation)> {
        self.ranges
            .iter()
            .find(|range| range.network.contains(&ip))
            .map(|range| {
                (
                    GeoNetworkRelation {
                        source_code: "personal_database".to_string(),
                        registry: None,
                        network: Some(range.network.to_string()),
                        start_ip: None,
                        end_ip: None,
                        asn: None,
                        organization: None,
                        source_reference: None,
                        network_kind: Some(range.network_kind),
                        risk_score: Some(range.risk_score),
                        risk_labels: range.risk_labels.clone(),
                    },
                    range.location.clone(),
                )
            })
    }
}

#[cfg(test)]
#[path = "region.geo.database.tests.rs"]
mod tests;
