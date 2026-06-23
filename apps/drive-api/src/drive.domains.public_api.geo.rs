use nvbes_region::geo::{
    GeoLookupRecordContext, GeoResolution, record_geo_resolution_tx, resolve_cached_geo_tx,
};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

pub struct ApiRequestGeo {
    pub country_code: Option<String>,
    pub source: &'static str,
    pub confidence: &'static str,
    pub network_kind: &'static str,
    pub risk_score: i16,
    pub risk_labels: Vec<String>,
}

pub async fn resolve_api_request_geo_tx(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    ip: Option<&str>,
    request_id: &str,
) -> Result<ApiRequestGeo, sqlx::Error> {
    let resolution = resolve_cached_geo_tx(tx, ip, None, None).await?;
    record_geo_resolution_tx(
        tx,
        GeoLookupRecordContext {
            purpose: "drive_api",
            subject_type: Some("workspace"),
            subject_id: Some(workspace_id),
            request_id: Some(request_id),
        },
        &resolution,
    )
    .await?;
    Ok(api_request_geo(&resolution))
}

fn api_request_geo(resolution: &GeoResolution) -> ApiRequestGeo {
    ApiRequestGeo {
        country_code: resolution
            .location
            .as_ref()
            .map(|location| location.country_code.clone()),
        source: resolution.source.as_str(),
        confidence: resolution.confidence.as_str(),
        network_kind: resolution.network_kind.as_str(),
        risk_score: i16::from(resolution.risk_score),
        risk_labels: resolution.risk_labels.clone(),
    }
}

#[cfg(test)]
mod tests {
    use nvbes_region::geo::{GeoConfidence, GeoEvidenceSource, GeoNetworkKind, GeoResolution};

    use super::api_request_geo;

    #[test]
    fn maps_geo_resolution_to_api_log_fields() {
        let fields = api_request_geo(&GeoResolution {
            location: None,
            confidence: GeoConfidence::Medium,
            source: GeoEvidenceSource::RemoteLookup,
            ip: None,
            private_network: false,
            network_kind: GeoNetworkKind::Datacenter,
            risk_score: 70,
            risk_labels: vec!["datacenter".to_string()],
            evidence: Vec::new(),
        });

        assert_eq!(fields.source, "remote_lookup");
        assert_eq!(fields.network_kind, "datacenter");
        assert_eq!(fields.risk_score, 70);
        assert_eq!(fields.risk_labels, vec!["datacenter"]);
    }
}
