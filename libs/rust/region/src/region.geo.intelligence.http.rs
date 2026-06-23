use std::net::IpAddr;
use std::time::Instant;

use reqwest::Client;
use serde_json::Value;
use thiserror::Error;

use crate::geo::{
    IpIntelligenceInput, IpIntelligenceLookup, normalize_ip_intelligence, types::GeoNetworkKind,
};

#[derive(Debug, Error)]
pub enum IpIntelligenceHttpError {
    #[error("ip intelligence request failed")]
    Request(#[from] reqwest::Error),
    #[error("ip intelligence provider returned no usable network signals")]
    Empty,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum IpIntelligenceProviderSpecError {
    #[error(
        "ip intelligence provider spec must be source|url_template|optional_authorization_header"
    )]
    InvalidFormat,
    #[error("ip intelligence provider source is required")]
    MissingSource,
    #[error("ip intelligence provider url template must include {{ip}}")]
    MissingIpPlaceholder,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpIntelligenceHttpProvider {
    pub source_code: String,
    pub url_template: String,
    pub authorization_header: Option<String>,
}

impl IpIntelligenceHttpProvider {
    pub fn url_for_ip(&self, ip: IpAddr) -> String {
        self.url_template.replace("{ip}", &ip.to_string())
    }
}

pub fn providers_from_specs(
    specs: &[String],
) -> Result<Vec<IpIntelligenceHttpProvider>, IpIntelligenceProviderSpecError> {
    specs.iter().map(|spec| provider_from_spec(spec)).collect()
}

pub fn provider_from_spec(
    spec: &str,
) -> Result<IpIntelligenceHttpProvider, IpIntelligenceProviderSpecError> {
    let parts = spec.splitn(3, '|').map(str::trim).collect::<Vec<_>>();
    if parts.len() < 2 {
        return Err(IpIntelligenceProviderSpecError::InvalidFormat);
    }
    let source_code = parts[0];
    let url_template = parts[1];
    if source_code.is_empty() {
        return Err(IpIntelligenceProviderSpecError::MissingSource);
    }
    if !url_template.contains("{ip}") {
        return Err(IpIntelligenceProviderSpecError::MissingIpPlaceholder);
    }

    Ok(IpIntelligenceHttpProvider {
        source_code: source_code.to_string(),
        url_template: url_template.to_string(),
        authorization_header: parts
            .get(2)
            .copied()
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned),
    })
}

#[derive(Debug, Clone)]
pub struct IpIntelligenceHttpClient {
    client: Client,
    providers: Vec<IpIntelligenceHttpProvider>,
}

impl IpIntelligenceHttpClient {
    pub fn new(client: Client, providers: Vec<IpIntelligenceHttpProvider>) -> Self {
        Self { client, providers }
    }

    pub async fn lookup(
        &self,
        ip: IpAddr,
    ) -> Result<Option<IpIntelligenceLookup>, IpIntelligenceHttpError> {
        for provider in &self.providers {
            if let Some(lookup) = self.lookup_provider(provider, ip).await? {
                return Ok(Some(lookup));
            }
        }
        Ok(None)
    }

    async fn lookup_provider(
        &self,
        provider: &IpIntelligenceHttpProvider,
        ip: IpAddr,
    ) -> Result<Option<IpIntelligenceLookup>, IpIntelligenceHttpError> {
        let started_at = Instant::now();
        let mut request = self.client.get(provider.url_for_ip(ip));
        if let Some(value) = provider.authorization_header.as_deref() {
            request = request.header(reqwest::header::AUTHORIZATION, value);
        }

        let response = match request.send().await {
            Ok(response) => response,
            Err(error) => {
                crate::geo::metrics::record_ip_intelligence_lookup(
                    provider.source_code.as_str(),
                    "request_error",
                    started_at.elapsed(),
                );
                return Err(error.into());
            }
        };
        if !response.status().is_success() {
            crate::geo::metrics::record_ip_intelligence_lookup(
                provider.source_code.as_str(),
                "http_miss",
                started_at.elapsed(),
            );
            return Ok(None);
        }
        let body = match response.json::<Value>().await {
            Ok(body) => body,
            Err(error) => {
                crate::geo::metrics::record_ip_intelligence_lookup(
                    provider.source_code.as_str(),
                    "decode_error",
                    started_at.elapsed(),
                );
                return Err(error.into());
            }
        };
        match normalize_http_body(provider.source_code.as_str(), ip, &body) {
            Ok(lookup) => {
                crate::geo::metrics::record_ip_intelligence_lookup(
                    provider.source_code.as_str(),
                    "hit",
                    started_at.elapsed(),
                );
                Ok(Some(lookup))
            }
            Err(IpIntelligenceHttpError::Empty) => {
                crate::geo::metrics::record_ip_intelligence_lookup(
                    provider.source_code.as_str(),
                    "empty",
                    started_at.elapsed(),
                );
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }
}

pub fn normalize_http_body(
    source_code: &str,
    ip: IpAddr,
    body: &Value,
) -> Result<IpIntelligenceLookup, IpIntelligenceHttpError> {
    let input = IpIntelligenceInput {
        source_code: source_code.to_string(),
        ip,
        country_code: first_string(body, &["country_code", "countryCode", "country"]),
        asn: first_i64(body, &["asn", "as_number", "asn_number"]),
        organization: first_string(body, &["organization", "org", "asn_org", "as_name", "isp"]),
        network: first_string(body, &["network", "cidr", "route", "prefix"]),
        source_reference: first_string(body, &["id", "request_id", "reference"]),
        is_vpn: first_bool(body, &["vpn", "is_vpn", "privacy.vpn"]),
        is_proxy: first_bool(body, &["proxy", "is_proxy", "privacy.proxy"]),
        is_tor: first_bool(body, &["tor", "is_tor", "privacy.tor"]),
        is_datacenter: first_bool(
            body,
            &[
                "datacenter",
                "is_datacenter",
                "hosting",
                "is_hosting",
                "privacy.hosting",
            ],
        ),
        is_mobile: first_bool(body, &["mobile", "is_mobile", "connection.mobile"]),
        is_residential: first_bool(body, &["residential", "is_residential"]),
        risk_score: first_i64(body, &["risk_score", "risk", "fraud_score"])
            .and_then(|value| u8::try_from(value).ok())
            .filter(|value| *value <= 100),
        risk_labels: labels_from_body(body),
    };

    let lookup = normalize_ip_intelligence(input);
    if lookup.relation.network_kind == Some(GeoNetworkKind::Unknown)
        && lookup.relation.risk_score == Some(50)
        && lookup.country_code.is_none()
    {
        return Err(IpIntelligenceHttpError::Empty);
    }
    Ok(lookup)
}

fn first_string(body: &Value, paths: &[&str]) -> Option<String> {
    paths
        .iter()
        .find_map(|path| {
            value_at_path(body, path)?
                .as_str()
                .map(|value| value.trim().to_string())
        })
        .filter(|value| !value.is_empty())
}

fn first_i64(body: &Value, paths: &[&str]) -> Option<i64> {
    paths.iter().find_map(|path| {
        let value = value_at_path(body, path)?;
        value
            .as_i64()
            .or_else(|| value.as_str()?.trim_start_matches("AS").parse().ok())
    })
}

fn first_bool(body: &Value, paths: &[&str]) -> bool {
    paths.iter().any(|path| {
        value_at_path(body, path).is_some_and(|value| {
            value
                .as_bool()
                .or_else(|| value.as_i64().map(|number| number != 0))
                .or_else(|| {
                    value.as_str().map(|text| {
                        matches!(text.to_ascii_lowercase().as_str(), "true" | "yes" | "1")
                    })
                })
                .unwrap_or(false)
        })
    })
}

fn labels_from_body(body: &Value) -> Vec<String> {
    body.get("risk_labels")
        .or_else(|| body.get("labels"))
        .and_then(Value::as_array)
        .map(|labels| {
            labels
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn value_at_path<'a>(body: &'a Value, path: &str) -> Option<&'a Value> {
    path.split('.')
        .try_fold(body, |value, segment| value.get(segment))
}

#[cfg(test)]
#[path = "region.geo.intelligence.http.tests.rs"]
mod tests;
