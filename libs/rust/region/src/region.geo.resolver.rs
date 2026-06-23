use std::net::IpAddr;

use crate::geo::{
    database::PersonalGeoDatabase,
    ip::is_private_or_special_ip,
    rdap::RdapLookup,
    types::{GeoConfidence, GeoEvidence, GeoEvidenceSource, GeoLocation, GeoResolution},
};

#[derive(Debug, Clone, Copy, Default)]
pub struct GeoLookupRequest<'a> {
    pub ip: Option<IpAddr>,
    pub trusted_country_header: Option<&'a str>,
    pub provider_country_header: Option<&'a str>,
    pub remote_lookup: Option<&'a RdapLookup>,
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
            evidence.push(GeoEvidence::accepted_with_relation(
                GeoEvidenceSource::RemoteLookup,
                &location,
                GeoConfidence::Medium,
                "remote rdap lookup provider",
                remote_lookup.relation.clone(),
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

fn resolved(
    location: GeoLocation,
    confidence: GeoConfidence,
    source: GeoEvidenceSource,
    ip: Option<IpAddr>,
    private_network: bool,
    evidence: Vec<GeoEvidence>,
) -> GeoResolution {
    let resolution = GeoResolution {
        location: Some(location),
        confidence,
        source,
        ip,
        private_network,
        evidence,
    };
    crate::geo::metrics::record_geo_resolution(&resolution);
    resolution
}

#[cfg(test)]
mod tests {
    use ipnet::IpNet;

    use super::{GeoLookupRequest, GeoResolver};
    use crate::geo::{
        database::{PersonalGeoDatabase, PersonalGeoRange},
        types::{GeoConfidence, GeoEvidenceSource},
    };

    #[test]
    fn trusted_header_wins_before_database() {
        let db = PersonalGeoDatabase::new(vec![
            PersonalGeoRange::new("8.8.8.0/24".parse::<IpNet>().unwrap(), "US").unwrap(),
        ]);
        let resolver = GeoResolver::new(db);

        let result = resolver.resolve(GeoLookupRequest {
            ip: Some("8.8.8.8".parse().unwrap()),
            trusted_country_header: Some("FR"),
            ..GeoLookupRequest::default()
        });

        assert_eq!(result.location.unwrap().country_code, "FR");
        assert_eq!(result.source, GeoEvidenceSource::TrustedProxyHeader);
    }

    #[test]
    fn database_wins_before_provider_header() {
        let db = PersonalGeoDatabase::new(vec![
            PersonalGeoRange::new("8.8.8.0/24".parse::<IpNet>().unwrap(), "US").unwrap(),
        ]);
        let resolver = GeoResolver::new(db);

        let result = resolver.resolve(GeoLookupRequest {
            ip: Some("8.8.8.8".parse().unwrap()),
            provider_country_header: Some("FR"),
            ..GeoLookupRequest::default()
        });

        assert_eq!(result.location.unwrap().country_code, "US");
        assert_eq!(result.source, GeoEvidenceSource::PersonalDatabase);
    }

    #[test]
    fn private_ip_can_only_use_non_ip_fallback() {
        let resolver = GeoResolver::default();

        let result = resolver.resolve(GeoLookupRequest {
            ip: Some("10.0.0.8".parse().unwrap()),
            stored_profile_country: Some("CH"),
            ..GeoLookupRequest::default()
        });

        assert_eq!(result.location.unwrap().country_code, "CH");
        assert_eq!(result.confidence, GeoConfidence::Low);
        assert!(result.private_network);
    }
}
