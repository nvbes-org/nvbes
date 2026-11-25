use serde::{Deserialize, Serialize};

use crate::geo::types::{
    GeoNetworkKind, GeoNetworkRelation, GeoRiskSignal, canonicalize_geo_risk_labels,
    push_unique_label,
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
        let explicit = GeoReputation {
            network_kind: kind,
            risk_score: score,
            risk_labels: canonical_labels_or_kind(&relation.risk_labels, kind),
        };
        if explicit.network_kind != GeoNetworkKind::Unknown || has_meaningful_label(&explicit) {
            return explicit;
        }

        let inferred = classify_text(&relation_text(relation));
        if inferred.network_kind != GeoNetworkKind::Unknown && inferred.risk_score > score {
            return merge_source_labels(inferred, &explicit.risk_labels);
        }

        return explicit;
    }

    let haystack = relation_text(relation);
    classify_text(&haystack)
}

fn has_meaningful_label(reputation: &GeoReputation) -> bool {
    reputation
        .risk_labels
        .iter()
        .any(|label| label != "unknown" && !label.starts_with("source:"))
}

fn merge_source_labels(mut reputation: GeoReputation, explicit_labels: &[String]) -> GeoReputation {
    for label in explicit_labels {
        if label.starts_with("source:") {
            push_unique_label(&mut reputation.risk_labels, label.clone());
        }
    }
    reputation
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
#[path = "region.geo.reputation.tests.rs"]
mod tests;
