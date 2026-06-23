use serde::{Deserialize, Serialize};

use crate::geo::types::{
    GeoNetworkKind, GeoNetworkRelation, GeoRiskSignal, canonicalize_geo_risk_labels,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeoReputation {
    pub network_kind: GeoNetworkKind,
    pub risk_score: u8,
    pub risk_labels: Vec<String>,
}

impl GeoReputation {
    pub fn trusted_residential() -> Self {
        Self {
            network_kind: GeoNetworkKind::Residential,
            risk_score: 15,
            risk_labels: vec!["residential".to_string()],
        }
    }

    pub fn private_network() -> Self {
        Self {
            network_kind: GeoNetworkKind::Unknown,
            risk_score: 0,
            risk_labels: vec!["private_network".to_string()],
        }
    }
}

pub fn score_relation(relation: &GeoNetworkRelation) -> GeoReputation {
    if let (Some(kind), Some(score)) = (relation.network_kind, relation.risk_score) {
        return GeoReputation {
            network_kind: kind,
            risk_score: score,
            risk_labels: canonical_labels_or_kind(&relation.risk_labels, kind),
        };
    }

    let haystack = relation_text(relation);
    classify_text(&haystack)
}

fn classify_text(value: &str) -> GeoReputation {
    let labels = detect_signals(value);
    let network_kind = kind_from_labels(&labels);
    let risk_score = score_for_kind(network_kind);

    GeoReputation {
        network_kind,
        risk_score,
        risk_labels: labels
            .into_iter()
            .map(|label| label.as_str().to_string())
            .collect(),
    }
}

fn relation_text(relation: &GeoNetworkRelation) -> String {
    [
        relation.organization.as_deref(),
        relation.source_reference.as_deref(),
        relation.registry.as_deref(),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" ")
    .to_ascii_lowercase()
}

fn detect_signals(value: &str) -> Vec<GeoRiskSignal> {
    let mut labels = Vec::new();
    push_if_any(
        &mut labels,
        value,
        GeoRiskSignal::Vpn,
        &[
            " vpn",
            "vpn ",
            "openvpn",
            "wireguard",
            "mullvad",
            "nordvpn",
            "proton vpn",
        ],
    );
    push_if_any(
        &mut labels,
        value,
        GeoRiskSignal::Proxy,
        &["proxy", "socks", "webshare", "bright data", "luminati"],
    );
    push_if_any(
        &mut labels,
        value,
        GeoRiskSignal::Tor,
        &[" tor ", "tor-exit", "tor exit"],
    );
    push_if_any(
        &mut labels,
        value,
        GeoRiskSignal::Datacenter,
        &[
            "hosting",
            "data center",
            "datacenter",
            "cloud",
            "amazon",
            "aws",
            "google",
            "azure",
            "microsoft",
            "digitalocean",
            "ovh",
            "hetzner",
            "linode",
            "leaseweb",
            "colo",
            "server",
        ],
    );
    push_if_any(
        &mut labels,
        value,
        GeoRiskSignal::Mobile,
        &["mobile", "cellular", "wireless", "lte", "5g", "4g"],
    );
    push_if_any(
        &mut labels,
        value,
        GeoRiskSignal::Residential,
        &[
            "broadband",
            "fiber",
            "fibre",
            "cable",
            "dsl",
            "residential",
            "telecom",
        ],
    );

    if labels.is_empty() {
        labels.push(GeoRiskSignal::Unknown);
    }
    labels
}

fn push_if_any(
    labels: &mut Vec<GeoRiskSignal>,
    value: &str,
    label: GeoRiskSignal,
    needles: &[&str],
) {
    if needles.iter().any(|needle| value.contains(needle)) && !labels.contains(&label) {
        labels.push(label);
    }
}

fn kind_from_labels(labels: &[GeoRiskSignal]) -> GeoNetworkKind {
    if labels.contains(&GeoRiskSignal::Tor) {
        return GeoNetworkKind::Tor;
    }
    if labels.contains(&GeoRiskSignal::Vpn) {
        return GeoNetworkKind::Vpn;
    }
    if labels.contains(&GeoRiskSignal::Proxy) {
        return GeoNetworkKind::Proxy;
    }
    if labels.contains(&GeoRiskSignal::Datacenter) {
        return GeoNetworkKind::Datacenter;
    }
    if labels.contains(&GeoRiskSignal::Mobile) {
        return GeoNetworkKind::Mobile;
    }
    if labels.contains(&GeoRiskSignal::Residential) {
        return GeoNetworkKind::Residential;
    }
    GeoNetworkKind::Unknown
}

fn score_for_kind(kind: GeoNetworkKind) -> u8 {
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

fn canonical_labels_or_kind(labels: &[String], kind: GeoNetworkKind) -> Vec<String> {
    let mut labels = canonicalize_geo_risk_labels(labels.iter().cloned());
    if labels.is_empty() {
        labels.push(kind.as_str().to_string());
    }
    labels
}

#[cfg(test)]
mod tests {
    use super::score_relation;
    use crate::geo::types::{GeoNetworkKind, GeoNetworkRelation};

    fn relation(organization: &str) -> GeoNetworkRelation {
        GeoNetworkRelation {
            source_code: "arin".to_string(),
            registry: Some("arin".to_string()),
            network: Some("203.0.113.0/24".to_string()),
            start_ip: None,
            end_ip: None,
            asn: Some(64500),
            organization: Some(organization.to_string()),
            source_reference: None,
            network_kind: None,
            risk_score: None,
            risk_labels: Vec::new(),
        }
    }

    #[test]
    fn scores_datacenter_and_vpn_higher_than_residential() {
        assert_eq!(
            score_relation(&relation("Amazon Web Services")).network_kind,
            GeoNetworkKind::Datacenter
        );
        assert_eq!(
            score_relation(&relation("Mullvad VPN")).network_kind,
            GeoNetworkKind::Vpn
        );
        assert!(
            score_relation(&relation("Mullvad VPN")).risk_score
                > score_relation(&relation("Example Broadband Telecom")).risk_score
        );
    }

    #[test]
    fn explicit_relation_labels_are_canonicalized() {
        let mut relation = relation("Example Provider");
        relation.network_kind = Some(GeoNetworkKind::Vpn);
        relation.risk_score = Some(90);
        relation.risk_labels = vec![
            "commercial_vpn".to_string(),
            "is_vpn".to_string(),
            "hosting".to_string(),
        ];

        let reputation = score_relation(&relation);

        assert_eq!(reputation.risk_labels, vec!["vpn", "datacenter"]);
    }
}
