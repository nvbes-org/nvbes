use std::net::IpAddr;

use reqwest::Client;
use serde_json::Value;
use sqlx::{Postgres, Transaction};
use thiserror::Error;

use super::maxmind_types::{
    MAXMIND_GEOLITE_WEB_SOURCE_CODE, MaxMindGeoLiteConfig, MaxMindGeoLiteConfigError,
    maxmind_ip_relation,
};
use crate::geo::{
    GeoConfidence, GeoEvidenceSource, GeoLocation, RdapLookup, types::GeoNetworkRelation,
};

const GEOLITE_CITY_ENDPOINT: &str = "https://geolite.info/geoip/v2.1/city";

#[derive(Debug, Error)]
pub enum MaxMindGeoLiteWebError {
    #[error("MaxMind GeoLite config is invalid")]
    Config(#[from] MaxMindGeoLiteConfigError),
    #[error("MaxMind GeoLite web request failed")]
    Request(#[from] reqwest::Error),
    #[error("MaxMind GeoLite web response did not include a supported country")]
    CountryMissing,
    #[error("MaxMind GeoLite web cache write failed")]
    Database(#[from] sqlx::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaxMindGeoLiteWebLookup {
    pub location: GeoLocation,
    pub relation: GeoNetworkRelation,
}

impl MaxMindGeoLiteWebLookup {
    pub fn as_rdap_lookup(&self) -> RdapLookup {
        RdapLookup {
            location: self.location.clone(),
            relation: self.relation.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MaxMindGeoLiteWebClient {
    client: Client,
    config: MaxMindGeoLiteConfig,
}

impl MaxMindGeoLiteWebClient {
    pub fn new(
        client: Client,
        config: MaxMindGeoLiteConfig,
    ) -> Result<Self, MaxMindGeoLiteWebError> {
        Ok(Self {
            client,
            config: config.enabled()?,
        })
    }

    pub async fn lookup(
        &self,
        ip: IpAddr,
    ) -> Result<Option<MaxMindGeoLiteWebLookup>, MaxMindGeoLiteWebError> {
        let response = self
            .client
            .get(format!("{GEOLITE_CITY_ENDPOINT}/{ip}"))
            .basic_auth(&self.config.account_id, Some(&self.config.license_key))
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }

        let body = response.error_for_status()?.json::<Value>().await?;
        let Some((relation, location)) = maxmind_ip_relation(ip, &body) else {
            return Err(MaxMindGeoLiteWebError::CountryMissing);
        };
        Ok(Some(MaxMindGeoLiteWebLookup { location, relation }))
    }
}

pub async fn cache_maxmind_geolite_web_tx(
    tx: &mut Transaction<'_, Postgres>,
    lookup: &MaxMindGeoLiteWebLookup,
    expires_at: chrono::DateTime<chrono::Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO geo_ip_network_relations (
          source_code, relation_key, registry, network, start_ip, end_ip, asn,
          organization, country_code, source_reference, network_kind, risk_score,
          risk_labels, raw_payload, fetched_at, expires_at
        )
        VALUES (
          $1, $2, $3, $4::cidr, $5::inet, $6::inet, $7,
          $8, $9, $10, $11, $12, $13,
          jsonb_build_object('source', $1, 'confidence', $14),
          now(), $15
        )
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
          raw_payload = EXCLUDED.raw_payload,
          fetched_at = now(),
          expires_at = EXCLUDED.expires_at
        "#,
    )
    .bind(MAXMIND_GEOLITE_WEB_SOURCE_CODE)
    .bind(relation_key(&lookup.relation))
    .bind(lookup.relation.registry.as_deref())
    .bind(lookup.relation.network.as_deref())
    .bind(lookup.relation.start_ip.map(|ip| ip.to_string()))
    .bind(lookup.relation.end_ip.map(|ip| ip.to_string()))
    .bind(lookup.relation.asn)
    .bind(lookup.relation.organization.as_deref())
    .bind(lookup.location.country_code.as_str())
    .bind(lookup.relation.source_reference.as_deref())
    .bind(lookup.relation.network_kind.map(|kind| kind.as_str()))
    .bind(lookup.relation.risk_score.map(i16::from))
    .bind(&lookup.relation.risk_labels)
    .bind(GeoConfidence::Medium.as_str())
    .bind(expires_at)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

fn relation_key(relation: &GeoNetworkRelation) -> String {
    relation
        .network
        .as_ref()
        .map(|network| format!("{}:{network}", relation.source_code))
        .unwrap_or_else(|| relation.source_code.clone())
}

pub fn maxmind_web_evidence_source() -> GeoEvidenceSource {
    GeoEvidenceSource::RemoteLookup
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::maxmind_ip_relation;

    #[test]
    fn maps_country_response_to_relation() {
        let body = json!({"country": {"iso_code": "FR"}});
        let (relation, location) = maxmind_ip_relation("203.0.113.7".parse().unwrap(), &body)
            .expect("relation should parse");

        assert_eq!(location.country_code, "FR");
        assert_eq!(relation.network.as_deref(), Some("203.0.113.7/32"));
    }
}
