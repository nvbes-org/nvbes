use std::net::IpAddr;

use serde::{Deserialize, Serialize};

use crate::geo::{
    reputation::score_relation,
    types::{GeoNetworkKind, GeoNetworkRelation},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IpIntelligenceInput {
    pub source_code: String,
    pub ip: IpAddr,
    pub country_code: Option<String>,
    pub asn: Option<i64>,
    pub organization: Option<String>,
    pub network: Option<String>,
    pub source_reference: Option<String>,
    pub is_vpn: bool,
    pub is_proxy: bool,
    pub is_tor: bool,
    pub is_datacenter: bool,
    pub is_mobile: bool,
    pub is_residential: bool,
    pub risk_score: Option<u8>,
    pub risk_labels: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IpIntelligenceLookup {
    pub country_code: Option<String>,
    pub relation: GeoNetworkRelation,
}

pub fn normalize_ip_intelligence(input: IpIntelligenceInput) -> IpIntelligenceLookup {
    let network_kind = classify_input(&input);
    let provider_code = input.source_code;
    let mut labels = input.risk_labels;
    push_label(&mut labels, network_kind.as_str());
    push_source_label(&mut labels, &provider_code);
    let risk_score = input
        .risk_score
        .unwrap_or_else(|| default_score(network_kind));

    IpIntelligenceLookup {
        country_code: input
            .country_code
            .map(|country| country.to_ascii_uppercase()),
        relation: GeoNetworkRelation {
            source_code: "ip_intelligence".to_string(),
            registry: None,
            network: input.network,
            start_ip: Some(input.ip),
            end_ip: Some(input.ip),
            asn: input.asn,
            organization: input.organization,
            source_reference: input.source_reference.or(Some(provider_code)),
            network_kind: Some(network_kind),
            risk_score: Some(risk_score),
            risk_labels: labels,
        },
    }
}

pub fn merge_intelligence_into_relation(
    mut relation: GeoNetworkRelation,
    intelligence: &IpIntelligenceLookup,
) -> GeoNetworkRelation {
    let intelligence_reputation = score_relation(&intelligence.relation);
    let current_reputation = score_relation(&relation);

    if intelligence_reputation.risk_score >= current_reputation.risk_score {
        relation.network_kind = Some(intelligence_reputation.network_kind);
        relation.risk_score = Some(intelligence_reputation.risk_score);
        relation.risk_labels = intelligence_reputation.risk_labels;
    }
    relation
}

fn classify_input(input: &IpIntelligenceInput) -> GeoNetworkKind {
    if input.is_tor {
        return GeoNetworkKind::Tor;
    }
    if input.is_vpn {
        return GeoNetworkKind::Vpn;
    }
    if input.is_proxy {
        return GeoNetworkKind::Proxy;
    }
    if input.is_datacenter {
        return GeoNetworkKind::Datacenter;
    }
    if input.is_mobile {
        return GeoNetworkKind::Mobile;
    }
    if input.is_residential {
        return GeoNetworkKind::Residential;
    }
    GeoNetworkKind::Unknown
}

fn default_score(kind: GeoNetworkKind) -> u8 {
    match kind {
        GeoNetworkKind::Tor => 95,
        GeoNetworkKind::Vpn => 90,
        GeoNetworkKind::Proxy => 85,
        GeoNetworkKind::Datacenter => 70,
        GeoNetworkKind::Unknown => 50,
        GeoNetworkKind::Mobile => 35,
        GeoNetworkKind::Residential => 15,
    }
}

fn push_label(labels: &mut Vec<String>, label: &str) {
    if !labels.iter().any(|item| item == label) {
        labels.push(label.to_string());
    }
}

fn push_source_label(labels: &mut Vec<String>, source_code: &str) {
    let label = format!("source:{source_code}");
    if !labels.iter().any(|item| item == &label) {
        labels.push(label);
    }
}

#[cfg(test)]
mod tests {
    use super::{IpIntelligenceInput, normalize_ip_intelligence};
    use crate::geo::types::GeoNetworkKind;

    #[test]
    fn normalizes_vpn_signal_with_default_score() {
        let lookup = normalize_ip_intelligence(IpIntelligenceInput {
            source_code: "test_provider".to_string(),
            ip: "203.0.113.42".parse().unwrap(),
            country_code: Some("fr".to_string()),
            asn: Some(64500),
            organization: Some("Example VPN".to_string()),
            network: Some("203.0.113.0/24".to_string()),
            source_reference: Some("fixture".to_string()),
            is_vpn: true,
            is_proxy: false,
            is_tor: false,
            is_datacenter: false,
            is_mobile: false,
            is_residential: false,
            risk_score: None,
            risk_labels: Vec::new(),
        });

        assert_eq!(lookup.country_code.as_deref(), Some("FR"));
        assert_eq!(lookup.relation.source_code, "ip_intelligence");
        assert_eq!(lookup.relation.network_kind, Some(GeoNetworkKind::Vpn));
        assert_eq!(lookup.relation.risk_score, Some(90));
        assert!(lookup.relation.risk_labels.contains(&"vpn".to_string()));
        assert!(
            lookup
                .relation
                .risk_labels
                .contains(&"source:test_provider".to_string())
        );
    }
}
