use sqlx::{Postgres, Row, Transaction};
use std::time::Instant;

use crate::geo::{
    GeoResolver, IpIntelligenceLookup,
    intelligence::merge_intelligence_into_relation,
    ip::{is_private_or_special_ip, parse_ip},
    persistence::load_personal_geo_database_tx,
    rdap::RdapLookup,
    reputation::score_relation,
    resolver::GeoLookupRequest,
    types::{GeoLocation, GeoNetworkKind, GeoNetworkRelation, GeoResolution},
};

pub async fn cached_remote_lookup_tx(
    tx: &mut Transaction<'_, Postgres>,
    ip: std::net::IpAddr,
) -> Result<Option<RdapLookup>, sqlx::Error> {
    let started_at = Instant::now();
    let row = cached_relation_row_tx(tx, ip, true).await?;

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
        relation: relation_from_row(&row)?,
    }))
}

pub async fn cached_ip_intelligence_tx(
    tx: &mut Transaction<'_, Postgres>,
    ip: std::net::IpAddr,
) -> Result<Option<IpIntelligenceLookup>, sqlx::Error> {
    let row = cached_relation_row_tx(tx, ip, false).await?;
    let Some(row) = row else {
        return Ok(None);
    };

    Ok(Some(IpIntelligenceLookup {
        country_code: row.try_get::<Option<String>, _>("country_code")?,
        relation: relation_from_row(&row)?,
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
    let (cached_lookup, cached_intelligence) =
        match parsed_ip.filter(|ip| !is_private_or_special_ip(*ip)) {
            Some(ip) => (
                cached_remote_lookup_tx(tx, ip).await?,
                cached_ip_intelligence_tx(tx, ip).await?,
            ),
            None => (None, None),
        };

    Ok(
        GeoResolver::new(personal_database).resolve(GeoLookupRequest {
            ip: parsed_ip,
            trusted_country_header,
            remote_lookup: cached_lookup.as_ref(),
            network_intelligence: cached_intelligence.as_ref(),
            stored_profile_country,
            ..GeoLookupRequest::default()
        }),
    )
}

async fn cached_relation_row_tx(
    tx: &mut Transaction<'_, Postgres>,
    ip: std::net::IpAddr,
    require_country: bool,
) -> Result<Option<sqlx::postgres::PgRow>, sqlx::Error> {
    sqlx::query(
        r#"
        SELECT
          source_code, registry, network::text AS network,
          start_ip::text AS start_ip, end_ip::text AS end_ip,
          asn, organization, country_code, source_reference,
          network_kind, risk_score, risk_labels
        FROM geo_ip_network_relations
        WHERE ($2::boolean = FALSE OR country_code IS NOT NULL)
          AND (expires_at IS NULL OR expires_at > now())
          AND (
            (network IS NOT NULL AND network >>= $1::inet)
            OR (start_ip IS NOT NULL AND end_ip IS NOT NULL AND $1::inet BETWEEN start_ip AND end_ip)
          )
        ORDER BY
          CASE WHEN source_code = 'ip_intelligence' THEN 0 ELSE 1 END,
          risk_score DESC,
          fetched_at DESC
        LIMIT 1
        "#,
    )
    .bind(ip.to_string())
    .bind(require_country)
    .fetch_optional(&mut **tx)
    .await
}

fn relation_from_row(row: &sqlx::postgres::PgRow) -> Result<GeoNetworkRelation, sqlx::Error> {
    let mut relation = GeoNetworkRelation {
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
        network_kind: row
            .try_get::<Option<String>, _>("network_kind")?
            .as_deref()
            .map(GeoNetworkKind::from_label),
        risk_score: row
            .try_get::<Option<i16>, _>("risk_score")?
            .and_then(|score| u8::try_from(score).ok()),
        risk_labels: row.try_get::<Vec<String>, _>("risk_labels")?,
    };
    let reputation = score_relation(&relation);
    relation.network_kind = Some(reputation.network_kind);
    relation.risk_score = Some(reputation.risk_score);
    relation.risk_labels = reputation.risk_labels;
    Ok(relation)
}

pub fn merge_cached_intelligence(
    relation: GeoNetworkRelation,
    intelligence: Option<&IpIntelligenceLookup>,
) -> GeoNetworkRelation {
    intelligence
        .map(|value| merge_intelligence_into_relation(relation.clone(), value))
        .unwrap_or(relation)
}

#[cfg(test)]
#[path = "region.geo.cache.tests.rs"]
mod tests;
