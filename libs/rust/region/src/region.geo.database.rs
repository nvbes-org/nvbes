use std::net::IpAddr;

use ipnet::IpNet;
use thiserror::Error;

use crate::geo::types::{GeoLocation, GeoNetworkRelation};

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PersonalGeoDatabaseError {
    #[error("unsupported country code for personal geo range")]
    UnsupportedCountryCode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersonalGeoRange {
    network: IpNet,
    location: GeoLocation,
}

impl PersonalGeoRange {
    pub fn new(network: IpNet, country_code: &str) -> Result<Self, PersonalGeoDatabaseError> {
        let location = GeoLocation::from_country_code(country_code)
            .ok_or(PersonalGeoDatabaseError::UnsupportedCountryCode)?;
        Ok(Self { network, location })
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
                    },
                    range.location.clone(),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::{PersonalGeoDatabase, PersonalGeoRange};

    #[test]
    fn matches_cidr_range() {
        let range = PersonalGeoRange::new("203.0.113.0/24".parse().unwrap(), "FR").unwrap();
        let db = PersonalGeoDatabase::new(vec![range]);

        let location = db.lookup("203.0.113.42".parse().unwrap()).unwrap();
        assert_eq!(location.country_code, "FR");
        assert!(db.lookup("198.51.100.1".parse().unwrap()).is_none());
    }

    #[test]
    fn rejects_unsupported_country_code() {
        let result = PersonalGeoRange::new("203.0.113.0/24".parse().unwrap(), "XX");
        assert!(result.is_err());
    }
}
