use ipnet::IpNet;
use sqlx::{Postgres, Row, Transaction};
use std::time::Instant;
use uuid::Uuid;

use crate::geo::{
    database::{PersonalGeoDatabase, PersonalGeoRange},
    ip::{is_private_or_special_ip, parse_ip},
    rdap::RdapLookup,
    resolver::{GeoLookupRequest, GeoResolver},
    types::{GeoEvidence, GeoLocation, GeoNetworkRelation, GeoResolution},
};

#[derive(Debug, Clone, Copy)]
pub struct GeoLookupRecordContext<'a> {
    pub purpose: &'a str,
    pub subject_type: Option<&'a str>,
    pub subject_id: Option<Uuid>,
    pub request_id: Option<&'a str>,
}

pub async fn record_geo_resolution_tx(
    tx: &mut Transaction<'_, Postgres>,
    context: GeoLookupRecordContext<'_>,
    resolution: &GeoResolution,
) -> Result<Uuid, sqlx::Error> {
    let location = resolution.location.as_ref();
    let event_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO geo_lookup_events (
          purpose, subject_type, subject_id, request_id, ip_address,
          selected_country_code, selected_data_region, selected_legal_jurisdiction,
          selected_source, confidence, private_network
        )
        VALUES ($1, $2, $3, $4, $5::inet, $6, $7, $8, $9, $10, $11)
        RETURNING id
        "#,
    )
    .bind(context.purpose)
    .bind(context.subject_type)
    .bind(context.subject_id)
    .bind(context.request_id)
    .bind(resolution.ip.map(|ip| ip.to_string()))
    .bind(location.map(|location| location.country_code.as_str()))
    .bind(location.map(|location| location.data_region.as_str()))
    .bind(location.map(|location| location.legal_jurisdiction.as_str()))
    .bind(resolution.source.as_str())
    .bind(resolution.confidence.as_str())
    .bind(resolution.private_network)
    .fetch_one(&mut **tx)
    .await?;

    for evidence in &resolution.evidence {
        record_evidence_tx(tx, event_id, evidence).await?;
    }

    Ok(event_id)
}

pub async fn load_personal_geo_database_tx(
    tx: &mut Transaction<'_, Postgres>,
) -> Result<PersonalGeoDatabase, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT network::text AS network, country_code
        FROM geo_personal_ip_ranges
        WHERE enabled = TRUE
          AND (expires_at IS NULL OR expires_at > now())
        ORDER BY priority ASC, masklen(network) DESC
        "#,
    )
    .fetch_all(&mut **tx)
    .await?;

    let ranges = rows
        .into_iter()
        .filter_map(|row| {
            let network = row.try_get::<String, _>("network").ok()?;
            let country_code = row.try_get::<String, _>("country_code").ok()?;
            let network = network.parse::<IpNet>().ok()?;
            PersonalGeoRange::new(network, &country_code).ok()
        })
        .collect();

    Ok(PersonalGeoDatabase::new(ranges))
}

pub async fn cached_remote_lookup_tx(
    tx: &mut Transaction<'_, Postgres>,
    ip: std::net::IpAddr,
) -> Result<Option<RdapLookup>, sqlx::Error> {
    let started_at = Instant::now();
    let row = sqlx::query(
        r#"
        SELECT
          source_code, registry, network::text AS network,
          start_ip::text AS start_ip, end_ip::text AS end_ip,
          asn, organization, country_code, source_reference
        FROM geo_ip_network_relations
        WHERE country_code IS NOT NULL
          AND (expires_at IS NULL OR expires_at > now())
          AND (
            (network IS NOT NULL AND network >>= $1::inet)
            OR (start_ip IS NOT NULL AND end_ip IS NOT NULL AND $1::inet BETWEEN start_ip AND end_ip)
          )
        ORDER BY fetched_at DESC
        LIMIT 1
        "#,
    )
    .bind(ip.to_string())
    .fetch_optional(&mut **tx)
    .await?;

    let Some(row) = row else {
        crate::geo::metrics::record_geo_cache_lookup("miss", started_at.elapsed());
        return Ok(None);
    };
    let country_code = row.try_get::<String, _>("country_code")?;
    let Some(location) = GeoLocation::from_country_code(&country_code) else {
        crate::geo::metrics::record_geo_cache_lookup("invalid_country", started_at.elapsed());
        return Ok(None);
    };

    crate::geo::metrics::record_geo_cache_lookup("hit", started_at.elapsed());
    Ok(Some(RdapLookup {
        location,
        relation: GeoNetworkRelation {
            source_code: row.try_get("source_code")?,
            registry: row.try_get("registry")?,
            network: row.try_get("network")?,
            start_ip: row
                .try_get::<Option<String>, _>("start_ip")?
                .and_then(|value| value.parse().ok()),
            end_ip: row
                .try_get::<Option<String>, _>("end_ip")?
                .and_then(|value| value.parse().ok()),
            asn: row.try_get("asn")?,
            organization: row.try_get("organization")?,
            source_reference: row.try_get("source_reference")?,
        },
    }))
}

pub async fn resolve_cached_geo_tx(
    tx: &mut Transaction<'_, Postgres>,
    ip: Option<&str>,
    trusted_country_header: Option<&str>,
    stored_profile_country: Option<&str>,
) -> Result<GeoResolution, sqlx::Error> {
    let parsed_ip = ip.and_then(parse_ip);
    let personal_database = load_personal_geo_database_tx(tx).await?;
    let cached_lookup = match parsed_ip.filter(|ip| !is_private_or_special_ip(*ip)) {
        Some(ip) => cached_remote_lookup_tx(tx, ip).await?,
        None => None,
    };

    Ok(
        GeoResolver::new(personal_database).resolve(GeoLookupRequest {
            ip: parsed_ip,
            trusted_country_header,
            remote_lookup: cached_lookup.as_ref(),
            stored_profile_country,
            ..GeoLookupRequest::default()
        }),
    )
}

async fn record_evidence_tx(
    tx: &mut Transaction<'_, Postgres>,
    event_id: Uuid,
    evidence: &GeoEvidence,
) -> Result<(), sqlx::Error> {
    let relation_id = match &evidence.relation {
        Some(relation) => Some(upsert_network_relation_tx(tx, relation, evidence).await?),
        None => None,
    };
    let source_code = evidence
        .relation
        .as_ref()
        .map(|relation| relation.source_code.as_str())
        .unwrap_or_else(|| evidence.source.as_str());

    sqlx::query(
        r#"
        INSERT INTO geo_lookup_evidence (
          lookup_event_id, source_code, network_relation_id, country_code,
          confidence, accepted, reason, relation_snapshot
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
    )
    .bind(event_id)
    .bind(source_code)
    .bind(relation_id)
    .bind(evidence.country_code.as_deref())
    .bind(evidence.confidence.as_str())
    .bind(evidence.accepted)
    .bind(evidence.reason.as_str())
    .bind(serde_json::to_value(&evidence.relation).unwrap_or(serde_json::Value::Null))
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn upsert_network_relation_tx(
    tx: &mut Transaction<'_, Postgres>,
    relation: &GeoNetworkRelation,
    evidence: &GeoEvidence,
) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO geo_ip_network_relations (
          source_code, relation_key, registry, network, start_ip, end_ip, asn,
          organization, country_code, source_reference
        )
        VALUES ($1, $2, $3, $4::cidr, $5::inet, $6::inet, $7, $8, $9, $10)
        ON CONFLICT (source_code, relation_key)
        DO UPDATE SET
          registry = EXCLUDED.registry,
          network = EXCLUDED.network,
          start_ip = EXCLUDED.start_ip,
          end_ip = EXCLUDED.end_ip,
          asn = EXCLUDED.asn,
          organization = EXCLUDED.organization,
          country_code = EXCLUDED.country_code,
          fetched_at = now()
        RETURNING id
        "#,
    )
    .bind(relation.source_code.as_str())
    .bind(relation_key(relation))
    .bind(relation.registry.as_deref())
    .bind(relation.network.as_deref())
    .bind(relation.start_ip.map(|ip| ip.to_string()))
    .bind(relation.end_ip.map(|ip| ip.to_string()))
    .bind(relation.asn)
    .bind(relation.organization.as_deref())
    .bind(evidence.country_code.as_deref())
    .bind(relation.source_reference.as_deref())
    .fetch_one(&mut **tx)
    .await
}

fn relation_key(relation: &GeoNetworkRelation) -> String {
    if let Some(network) = &relation.network {
        return format!("network:{network}");
    }
    if let (Some(start_ip), Some(end_ip)) = (relation.start_ip, relation.end_ip) {
        return format!("range:{start_ip}-{end_ip}");
    }
    relation
        .source_reference
        .as_deref()
        .map(|reference| format!("ref:{reference}"))
        .unwrap_or_else(|| "unknown".to_string())
}
