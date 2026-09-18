use std::net::IpAddr;
use std::time::Instant;

use reqwest::Client;
use serde_json::Value;
use thiserror::Error;

use crate::geo::{
    reputation::score_relation,
    types::{GeoLocation, GeoNetworkRelation},
};

#[derive(Debug, Error)]
pub enum RdapLookupError {
    #[error("rdap request failed")]
    Request(#[from] reqwest::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RdapRegistry {
    Arin,
    Ripe,
    Apnic,
    Lacnic,
    Afrinic,
}

impl RdapRegistry {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Arin => "arin",
            Self::Ripe => "ripe",
            Self::Apnic => "apnic",
            Self::Lacnic => "lacnic",
            Self::Afrinic => "afrinic",
        }
    }

    pub const fn base_url(self) -> &'static str {
        match self {
            Self::Arin => "https://rdap.arin.net/registry/ip",
            Self::Ripe => "https://rdap.db.ripe.net/ip",
            Self::Apnic => "https://rdap.apnic.net/ip",
            Self::Lacnic => "https://rdap.lacnic.net/rdap/ip",
            Self::Afrinic => "https://rdap.afrinic.net/rdap/ip",
        }
    }
}

pub const DEFAULT_RDAP_REGISTRIES: [RdapRegistry; 5] = [
    RdapRegistry::Arin,
    RdapRegistry::Ripe,
    RdapRegistry::Apnic,
    RdapRegistry::Lacnic,
    RdapRegistry::Afrinic,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RdapLookup {
    pub location: GeoLocation,
    pub relation: GeoNetworkRelation,
}

#[derive(Debug, Clone)]
pub struct RdapClient {
    client: Client,
    registries: Vec<RdapRegistry>,
}

impl RdapClient {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            registries: DEFAULT_RDAP_REGISTRIES.to_vec(),
        }
    }

    pub fn with_registries(client: Client, registries: Vec<RdapRegistry>) -> Self {
        Self { client, registries }
    }

    pub async fn lookup(&self, ip: IpAddr) -> Result<Option<RdapLookup>, RdapLookupError> {
        for registry in &self.registries {
            if let Some(lookup) = self.lookup_registry(*registry, ip).await? {
                return Ok(Some(lookup));
            }
        }
        Ok(None)
    }

    async fn lookup_registry(
        &self,
        registry: RdapRegistry,
        ip: IpAddr,
    ) -> Result<Option<RdapLookup>, RdapLookupError> {
        let started_at = Instant::now();
        let response = match self
            .client
            .get(format!("{}/{}", registry.base_url(), ip))
            .send()
            .await
        {
            Ok(response) => response,
            Err(error) => {
                crate::geo::metrics::record_rdap_lookup(
                    registry.code(),
                    "request_error",
                    started_at.elapsed(),
                );
                return Err(error.into());
            }
        };
        if !response.status().is_success() {
            crate::geo::metrics::record_rdap_lookup(
                registry.code(),
                "http_miss",
                started_at.elapsed(),
            );
            return Ok(None);
        }

        let body = match response.json::<Value>().await {
            Ok(body) => body,
            Err(error) => {
                crate::geo::metrics::record_rdap_lookup(
                    registry.code(),
                    "decode_error",
                    started_at.elapsed(),
                );
                return Err(error.into());
            }
        };
        let country_code = string_field(&body, "country")
            .or_else(|| first_notice_country(&body))
            .and_then(|value| GeoLocation::from_country_code(&value));
        let Some(location) = country_code else {
            crate::geo::metrics::record_rdap_lookup(
                registry.code(),
                "country_miss",
                started_at.elapsed(),
            );
            return Ok(None);
        };

        let mut relation = GeoNetworkRelation {
            source_code: registry.code().to_string(),
            registry: Some(registry.code().to_string()),
            network: None,
            start_ip: string_field(&body, "startAddress").and_then(|value| value.parse().ok()),
            end_ip: string_field(&body, "endAddress").and_then(|value| value.parse().ok()),
            asn: string_field(&body, "handle").and_then(parse_asn),
            organization: string_field(&body, "name").or_else(|| first_entity_name(&body)),
            source_reference: string_field(&body, "handle"),
            network_kind: None,
            risk_score: None,
            risk_labels: Vec::new(),
        };
        let reputation = score_relation(&relation);
        relation.network_kind = Some(reputation.network_kind);
        relation.risk_score = Some(reputation.risk_score);
        relation.risk_labels = reputation.risk_labels;

        crate::geo::metrics::record_rdap_lookup(registry.code(), "hit", started_at.elapsed());
        Ok(Some(RdapLookup { location, relation }))
    }
}

fn string_field(body: &Value, field: &str) -> Option<String> {
    body.get(field)?
        .as_str()
        .map(|value| value.trim().to_string())
}

pub(crate) fn parse_asn(value: String) -> Option<i64> {
    value
        .trim_start_matches("AS")
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .ok()
}

fn first_notice_country(body: &Value) -> Option<String> {
    body.get("notices")?
        .as_array()?
        .iter()
        .find_map(|notice| string_field(notice, "country"))
}

fn first_entity_name(body: &Value) -> Option<String> {
    body.get("entities")?
        .as_array()?
        .iter()
        .find_map(|entity| string_field(entity, "handle"))
}

#[cfg(test)]
mod tests {
    use super::{first_entity_name, parse_asn};

    #[test]
    fn parses_asn_handle() {
        assert_eq!(parse_asn("AS13335".to_string()), Some(13335));
        assert_eq!(parse_asn("NET-8-8-8-0-1".to_string()), None);
    }

    #[test]
    fn extracts_first_entity_handle() {
        let body = serde_json::json!({"entities": [{"handle": "ORG-EXAMPLE"}]});
        assert_eq!(first_entity_name(&body).as_deref(), Some("ORG-EXAMPLE"));
    }
}
