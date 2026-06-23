use ipnet::IpNet;
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

use crate::geo::{
    database::{PersonalGeoDatabase, PersonalGeoRange},
    intelligence::network_relation_key,
    types::{GeoEvidence, GeoNetworkKind, GeoNetworkRelation, GeoResolution},
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
          , network_kind, risk_score, risk_labels
        )
        VALUES ($1, $2, $3, $4, $5::inet, $6, $7, $8, $9, $10, $11, $12, $13, $14)
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
    .bind(resolution.network_kind.as_str())
    .bind(resolution.risk_score as i16)
    .bind(&resolution.risk_labels)
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
        SELECT network::text AS network, country_code, network_kind, risk_score, risk_labels
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
            let network_kind = row
                .try_get::<String, _>("network_kind")
                .map(|value| GeoNetworkKind::from_str(&value))
                .unwrap_or(GeoNetworkKind::Residential);
            let risk_score = row
                .try_get::<i16, _>("risk_score")
                .ok()
                .and_then(|score| u8::try_from(score).ok())
                .unwrap_or(15);
            let risk_labels = row
                .try_get::<Vec<String>, _>("risk_labels")
                .unwrap_or_else(|_| vec!["personal_database".to_string()]);
            let network = network.parse::<IpNet>().ok()?;
            PersonalGeoRange::with_reputation(
                network,
                &country_code,
                network_kind,
                risk_score,
                risk_labels,
            )
            .ok()
        })
        .collect();

    Ok(PersonalGeoDatabase::new(ranges))
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
          organization, country_code, source_reference, network_kind, risk_score, risk_labels
        )
        VALUES ($1, $2, $3, $4::cidr, $5::inet, $6::inet, $7, $8, $9, $10, $11, $12, $13)
        ON CONFLICT (source_code, relation_key)
        DO UPDATE SET
          registry = EXCLUDED.registry,
          network = EXCLUDED.network,
          start_ip = EXCLUDED.start_ip,
          end_ip = EXCLUDED.end_ip,
          asn = EXCLUDED.asn,
          organization = EXCLUDED.organization,
          country_code = EXCLUDED.country_code,
          network_kind = EXCLUDED.network_kind,
          risk_score = EXCLUDED.risk_score,
          risk_labels = EXCLUDED.risk_labels,
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
    .bind(evidence.country_code.as_deref())
    .bind(relation.source_reference.as_deref())
    .bind(relation.network_kind.map(|kind| kind.as_str()))
    .bind(relation.risk_score.map(i16::from))
    .bind(&relation.risk_labels)
    .fetch_one(&mut **tx)
    .await
}
