use nvbes_region::geo::{
    GeoLookupRecordContext, GeoResolution, record_geo_resolution_tx, resolve_cached_geo_tx,
};
use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::http::error::AppError;

pub struct AuditRecordInput<'a> {
    pub tenant_id: Uuid,
    pub workspace_id: Option<Uuid>,
    pub actor_principal_id: Option<Uuid>,
    pub action: &'a str,
    pub target_type: &'a str,
    pub target_id: Option<Uuid>,
    pub ip: Option<&'a str>,
    pub user_agent: Option<&'a str>,
    pub metadata: Value,
}

pub async fn record_event(db: &PgPool, input: AuditRecordInput<'_>) -> Result<(), AppError> {
    let mut tx = db.begin().await?;
    record_event_tx(&mut tx, input).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn record_event_tx(
    tx: &mut Transaction<'_, Postgres>,
    input: AuditRecordInput<'_>,
) -> Result<(), AppError> {
    let metadata = enrich_audit_metadata_tx(
        tx,
        input.tenant_id,
        input.workspace_id,
        input.ip,
        input.action,
        input.metadata,
    )
    .await?;

    sqlx::query(
        r#"
        INSERT INTO audit_events (
          tenant_id,
          workspace_id,
          actor_principal_id,
          action,
          target_type,
          target_id,
          ip,
          user_agent,
          metadata
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7::inet, $8, $9)
        "#,
    )
    .bind(input.tenant_id)
    .bind(input.workspace_id)
    .bind(input.actor_principal_id)
    .bind(input.action)
    .bind(input.target_type)
    .bind(input.target_id)
    .bind(input.ip)
    .bind(input.user_agent)
    .bind(sqlx::types::Json(metadata))
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn enrich_audit_metadata_tx(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    workspace_id: Option<Uuid>,
    ip: Option<&str>,
    action: &str,
    metadata: Value,
) -> Result<Value, sqlx::Error> {
    let resolution = resolve_cached_geo_tx(tx, ip, None, None).await?;
    record_geo_resolution_tx(
        tx,
        GeoLookupRecordContext {
            purpose: "identity_audit",
            subject_type: Some(if workspace_id.is_some() {
                "workspace"
            } else {
                "tenant"
            }),
            subject_id: workspace_id.or(Some(tenant_id)),
            request_id: Some(action),
        },
        &resolution,
    )
    .await?;

    Ok(with_geo_metadata(metadata, &resolution))
}

fn with_geo_metadata(mut metadata: Value, resolution: &GeoResolution) -> Value {
    let geo = serde_json::json!({
        "geo_country_code": resolution.location.as_ref().map(|location| location.country_code.as_str()),
        "geo_source": resolution.source.as_str(),
        "geo_confidence": resolution.confidence.as_str(),
        "geo_private_network": resolution.private_network,
        "geo_network_kind": resolution.network_kind.as_str(),
        "geo_risk_score": resolution.risk_score,
        "geo_risk_labels": resolution.risk_labels,
    });

    match metadata.as_object_mut() {
        Some(object) => {
            object.insert("geo".to_string(), geo);
            metadata
        }
        None => serde_json::json!({
            "value": metadata,
            "geo": geo,
        }),
    }
}

#[cfg(test)]
mod tests {
    use nvbes_region::geo::{GeoConfidence, GeoEvidenceSource, GeoNetworkKind, GeoResolution};

    use super::with_geo_metadata;

    #[test]
    fn adds_geo_fields_to_identity_audit_metadata() {
        let metadata = with_geo_metadata(
            serde_json::json!({"event": "test"}),
            &GeoResolution {
                location: None,
                confidence: GeoConfidence::Low,
                source: GeoEvidenceSource::Fallback,
                ip: None,
                private_network: false,
                network_kind: GeoNetworkKind::Proxy,
                risk_score: 85,
                risk_labels: vec!["proxy".to_string()],
                evidence: Vec::new(),
            },
        );

        assert_eq!(metadata["event"], "test");
        assert_eq!(metadata["geo"]["geo_network_kind"], "proxy");
        assert_eq!(metadata["geo"]["geo_risk_score"], 85);
    }
}
