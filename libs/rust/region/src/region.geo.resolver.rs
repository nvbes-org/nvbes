use std::net::IpAddr;

use crate::geo::{
    database::PersonalGeoDatabase,
    intelligence::{IpIntelligenceLookup, merge_intelligence_into_relation},
    ip::is_private_or_special_ip,
    rdap::RdapLookup,
    reputation::{GeoReputation, score_relation},
    types::{GeoConfidence, GeoEvidence, GeoEvidenceSource, GeoLocation, GeoResolution},
};

#[derive(Debug, Clone, Copy, Default)]
pub struct GeoLookupRequest<'a> {
    pub ip: Option<IpAddr>,
    pub trusted_country_header: Option<&'a str>,
    pub provider_country_header: Option<&'a str>,
    pub remote_lookup: Option<&'a RdapLookup>,
    pub network_intelligence: Option<&'a IpIntelligenceLookup>,
    pub stored_profile_country: Option<&'a str>,
}

#[derive(Debug, Clone, Default)]
pub struct GeoResolver {
    personal_database: PersonalGeoDatabase,
}

impl GeoResolver {
    pub fn new(personal_database: PersonalGeoDatabase) -> Self {
        Self { personal_database }
    }

    pub fn resolve(&self, request: GeoLookupRequest<'_>) -> GeoResolution {
        let private_network = request.ip.map(is_private_or_special_ip).unwrap_or(false);
        let mut evidence = Vec::new();
        push_network_intelligence_evidence(request.network_intelligence, &mut evidence);

        if let Some(location) = country_signal(
            request.trusted_country_header,
            GeoEvidenceSource::TrustedProxyHeader,
            GeoConfidence::High,
            "trusted proxy country header",
            &mut evidence,
        ) {
            return resolved(
                location,
                GeoConfidence::High,
                GeoEvidenceSource::TrustedProxyHeader,
                request.ip,
                private_network,
                evidence,
            );
        }

        if let Some(ip) = request.ip.filter(|_| !private_network) {
            if let Some((relation, location)) = self.personal_database.lookup_relation(ip) {
                evidence.push(GeoEvidence::accepted_with_relation(
                    GeoEvidenceSource::PersonalDatabase,
                    &location,
                    GeoConfidence::High,
                    "personal geo database cidr match",
                    relation,
                ));
                return resolved(
                    location,
                    GeoConfidence::High,
                    GeoEvidenceSource::PersonalDatabase,
                    request.ip,
                    false,
                    evidence,
                );
            }
        } else if private_network {
            evidence.push(GeoEvidence::rejected(
                GeoEvidenceSource::PrivateNetwork,
                None,
                "private or special-purpose ip cannot identify a country",
            ));
        }

        if let Some(location) = country_signal(
            request.provider_country_header,
            GeoEvidenceSource::ProviderHeader,
            GeoConfidence::Medium,
            "provider country header",
            &mut evidence,
        ) {
            return resolved(
                location,
                GeoConfidence::Medium,
                GeoEvidenceSource::ProviderHeader,
                request.ip,
                private_network,
                evidence,
            );
        }

        if let Some(remote_lookup) = request.remote_lookup {
            let location = remote_lookup.location.clone();
            let relation = request
                .network_intelligence
                .map(|intelligence| {
                    merge_intelligence_into_relation(remote_lookup.relation.clone(), intelligence)
                })
                .unwrap_or_else(|| remote_lookup.relation.clone());
            evidence.push(GeoEvidence::accepted_with_relation(
                GeoEvidenceSource::RemoteLookup,
                &location,
                GeoConfidence::Medium,
                "remote rdap lookup provider",
                relation,
            ));
            return resolved(
                location,
                GeoConfidence::Medium,
                GeoEvidenceSource::RemoteLookup,
                request.ip,
                private_network,
                evidence,
            );
        }

        if let Some(location) = country_signal(
            request.stored_profile_country,
            GeoEvidenceSource::StoredProfile,
            GeoConfidence::Low,
            "stored profile country fallback",
            &mut evidence,
        ) {
            return resolved(
                location,
                GeoConfidence::Low,
                GeoEvidenceSource::StoredProfile,
                request.ip,
                private_network,
                evidence,
            );
        }

        let resolution = GeoResolution::unresolved(request.ip, private_network, evidence);
        crate::geo::metrics::record_geo_resolution(&resolution);
        resolution
    }
}

fn country_signal(
    country_code: Option<&str>,
    source: GeoEvidenceSource,
    confidence: GeoConfidence,
    reason: &'static str,
    evidence: &mut Vec<GeoEvidence>,
) -> Option<GeoLocation> {
    let country_code = country_code?;
    let location = GeoLocation::from_country_code(country_code);
    match &location {
        Some(location) => {
            evidence.push(GeoEvidence::accepted(source, location, confidence, reason))
        }
        None => evidence.push(GeoEvidence::rejected(
            source,
            Some(country_code),
            "unsupported country code",
        )),
    }
    location
}

fn push_network_intelligence_evidence(
    intelligence: Option<&IpIntelligenceLookup>,
    evidence: &mut Vec<GeoEvidence>,
) {
    let Some(intelligence) = intelligence else {
        return;
    };
    let country_code = intelligence.country_code.as_deref();
    match country_code.and_then(GeoLocation::from_country_code) {
        Some(location) => evidence.push(GeoEvidence::accepted_with_relation(
            GeoEvidenceSource::RemoteLookup,
            &location,
            GeoConfidence::Medium,
            "network intelligence provider",
            intelligence.relation.clone(),
        )),
        None => evidence.push(GeoEvidence {
            source: GeoEvidenceSource::RemoteLookup,
            country_code: country_code.map(ToOwned::to_owned),
            confidence: GeoConfidence::Medium,
            accepted: false,
            reason: "network intelligence provider reputation only".to_string(),
            relation: Some(intelligence.relation.clone()),
        }),
    }
}

fn resolved(
    location: GeoLocation,
    confidence: GeoConfidence,
    source: GeoEvidenceSource,
    ip: Option<IpAddr>,
    private_network: bool,
    evidence: Vec<GeoEvidence>,
) -> GeoResolution {
    let reputation = reputation_from_evidence(&evidence, private_network);
    let resolution = GeoResolution {
        location: Some(location),
        confidence,
        source,
        ip,
        private_network,
        network_kind: reputation.network_kind,
        risk_score: reputation.risk_score,
        risk_labels: reputation.risk_labels,
        evidence,
    };
    crate::geo::metrics::record_geo_resolution(&resolution);
    resolution
}

fn reputation_from_evidence(evidence: &[GeoEvidence], private_network: bool) -> GeoReputation {
    if private_network {
        return GeoReputation::private_network();
    }

    evidence
        .iter()
        .filter_map(|entry| entry.relation.as_ref().map(score_relation))
        .max_by_key(|reputation| reputation.risk_score)
        .unwrap_or_else(|| GeoReputation {
            network_kind: crate::geo::types::GeoNetworkKind::Unknown,
            risk_score: 50,
            risk_labels: vec!["unknown".to_string()],
        })
}

#[cfg(test)]
#[path = "region.geo.resolver.tests.rs"]
mod tests;
