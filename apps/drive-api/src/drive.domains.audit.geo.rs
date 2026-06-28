use nvbes_region::geo::{
    GeoLookupPurpose, GeoLookupRecordContext, GeoResolution, record_geo_resolution_tx,
    resolve_cached_geo_tx,
};
use serde_json::Value;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

pub async fn enrich_audit_metadata_tx(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    ip: Option<&str>,
    action: &str,
    metadata: Value,
) -> Result<Value, sqlx::Error> {
    let resolution = resolve_cached_geo_tx(tx, ip, None, None).await?;
    record_geo_resolution_tx(
        tx,
        GeoLookupRecordContext {
            purpose: GeoLookupPurpose::DriveAudit,
            subject_type: Some("workspace"),
            subject_id: Some(workspace_id),
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
    fn adds_geo_fields_to_audit_metadata() {
        let metadata = with_geo_metadata(
            serde_json::json!({"object_type": "file"}),
            &GeoResolution {
                location: None,
                confidence: GeoConfidence::Low,
                source: GeoEvidenceSource::Fallback,
                ip: None,
                private_network: false,
                network_kind: GeoNetworkKind::Vpn,
                risk_score: 90,
                risk_labels: vec!["vpn".to_string()],
                evidence: Vec::new(),
            },
        );

        assert_eq!(metadata["object_type"], "file");
        assert_eq!(metadata["geo"]["geo_network_kind"], "vpn");
        assert_eq!(metadata["geo"]["geo_risk_score"], 90);
    }
}
