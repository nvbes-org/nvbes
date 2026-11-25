use std::net::IpAddr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::geo::{
    reputation::score_relation,
    types::{GeoNetworkKind, GeoNetworkRelation, canonicalize_geo_risk_labels, push_unique_label},
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

pub async fn cache_ip_intelligence_tx(
    tx: &mut Transaction<'_, Postgres>,
    lookup: &IpIntelligenceLookup,
    expires_at: Option<DateTime<Utc>>,
) -> Result<Uuid, sqlx::Error> {
    let relation = &lookup.relation;
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO geo_ip_network_relations (
          source_code, relation_key, registry, network, start_ip, end_ip, asn,
          organization, country_code, source_reference, network_kind, risk_score,
          risk_labels, expires_at
        )
        VALUES ($1, $2, $3, $4::cidr, $5::inet, $6::inet, $7, $8, $9, $10, $11, $12, $13, $14)
        ON CONFLICT (source_code, relation_key)
        DO UPDATE SET
          registry = EXCLUDED.registry,
          network = EXCLUDED.network,
          start_ip = EXCLUDED.start_ip,
          end_ip = EXCLUDED.end_ip,
          asn = EXCLUDED.asn,
          organization = EXCLUDED.organization,
          country_code = EXCLUDED.country_code,
          source_reference = EXCLUDED.source_reference,
          network_kind = EXCLUDED.network_kind,
          risk_score = EXCLUDED.risk_score,
          risk_labels = EXCLUDED.risk_labels,
          expires_at = EXCLUDED.expires_at,
          fetched_at = now()
        RETURNING id
        "#,
    )
    .bind(relation.source_code.as_str())
    .bind(network_relation_key(relation))
    .bind(relation.registry.as_deref())
    .bind(relation.network.as_deref())
    .bind(relation.start_ip.map(|ip| ip.to_string()))
    .bind(relation.end_ip.map(|ip| ip.to_string()))
    .bind(relation.asn)
    .bind(relation.organization.as_deref())
    .bind(lookup.country_code.as_deref())
    .bind(relation.source_reference.as_deref())
    .bind(relation.network_kind.map(|kind| kind.as_str()))
    .bind(relation.risk_score.map(i16::from))
    .bind(&relation.risk_labels)
    .bind(expires_at)
    .fetch_one(&mut **tx)
    .await
}

pub fn normalize_ip_intelligence(input: IpIntelligenceInput) -> IpIntelligenceLookup {
    let network_kind = classify_input(&input);
    let provider_code = input.source_code;
    let mut labels = canonicalize_geo_risk_labels(input.risk_labels);
    push_unique_label(&mut labels, network_kind.as_str());
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

fn push_source_label(labels: &mut Vec<String>, source_code: &str) {
    push_unique_label(labels, format!("source:{source_code}"));
}

pub(crate) fn network_relation_key(relation: &GeoNetworkRelation) -> String {
    let provider_prefix = if relation.source_code == "ip_intelligence" {
        provider_key(relation)
            .map(|provider| format!("provider:{provider}:"))
            .unwrap_or_default()
    } else {
        String::new()
    };
    if let Some(network) = &relation.network {
        return format!("{provider_prefix}network:{network}");
    }
    if let (Some(start_ip), Some(end_ip)) = (relation.start_ip, relation.end_ip) {
        return format!("{provider_prefix}range:{start_ip}-{end_ip}");
    }
    relation
        .source_reference
        .as_deref()
        .map(|reference| format!("{provider_prefix}ref:{reference}"))
        .unwrap_or_else(|| "unknown".to_string())
}

fn provider_key(relation: &GeoNetworkRelation) -> Option<String> {
    relation
        .risk_labels
        .iter()
        .find_map(|label| label.strip_prefix("source:"))
        .or(relation.source_reference.as_deref())
        .map(ToOwned::to_owned)
}

#[cfg(test)]
#[path = "region.geo.intelligence.tests.rs"]
mod tests;
