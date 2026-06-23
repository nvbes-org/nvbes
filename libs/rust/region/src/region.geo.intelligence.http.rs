use std::net::IpAddr;

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
        let mut request = self.client.get(provider.url_for_ip(ip));
        if let Some(value) = provider.authorization_header.as_deref() {
            request = request.header(reqwest::header::AUTHORIZATION, value);
        }

        let response = request.send().await?;
        if !response.status().is_success() {
            return Ok(None);
        }
        let body = response.json::<Value>().await?;
        normalize_http_body(provider.source_code.as_str(), ip, &body).map(Some)
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
mod tests {
    use super::{
        IpIntelligenceHttpProvider, normalize_http_body, provider_from_spec, providers_from_specs,
    };
    use crate::geo::types::GeoNetworkKind;

    #[test]
    fn provider_url_replaces_ip_placeholder() {
        let provider = IpIntelligenceHttpProvider {
            source_code: "fixture".to_string(),
            url_template: "https://example.test/ip/{ip}".to_string(),
            authorization_header: None,
        };

        assert_eq!(
            provider.url_for_ip("8.8.8.8".parse().unwrap()),
            "https://example.test/ip/8.8.8.8"
        );
    }

    #[test]
    fn parses_provider_specs() {
        let providers = providers_from_specs(&[
            "ipinfo|https://example.test/{ip}|Bearer token".to_string(),
            "maxmind|https://risk.example/{ip}".to_string(),
        ])
        .unwrap();

        assert_eq!(providers.len(), 2);
        assert_eq!(providers[0].source_code, "ipinfo");
        assert_eq!(
            providers[0].authorization_header.as_deref(),
            Some("Bearer token")
        );
        assert!(provider_from_spec("broken").is_err());
        assert!(provider_from_spec("source|https://example.test/no-placeholder").is_err());
    }

    #[test]
    fn normalizes_common_privacy_payload() {
        let body = serde_json::json!({
            "country": "fr",
            "asn": "AS64500",
            "org": "Example Network",
            "privacy": {"vpn": true, "proxy": false, "tor": false, "hosting": false},
            "risk_labels": ["commercial_vpn"]
        });

        let lookup =
            normalize_http_body("fixture_provider", "8.8.8.8".parse().unwrap(), &body).unwrap();

        assert_eq!(lookup.country_code.as_deref(), Some("FR"));
        assert_eq!(lookup.relation.asn, Some(64500));
        assert_eq!(lookup.relation.network_kind, Some(GeoNetworkKind::Vpn));
        assert_eq!(lookup.relation.risk_score, Some(90));
        assert!(
            lookup
                .relation
                .risk_labels
                .contains(&"commercial_vpn".to_string())
        );
    }
}
