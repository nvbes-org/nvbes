use nvbes_core::config::AppConfig;
use nvbes_region::geo::{
    GeoLookupPurpose, GeoLookupRecordContext, GeoNetworkKind, GeoResolution,
    record_geo_resolution_tx, resolve_cached_geo_tx,
};
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

pub struct GeoSecuritySignal {
    pub score: f64,
    pub decision: super::RiskDecision,
    pub factors: Value,
    pub metadata: Value,
}

pub async fn apply_geo_security_signal(
    db: &PgPool,
    config: &AppConfig,
    principal_id: Uuid,
    ip: Option<&str>,
    base_score: f64,
    mut factors: Value,
    event_type: &'static str,
) -> GeoSecuritySignal {
    let Some(resolution) = resolve_and_record(db, config, principal_id, ip, event_type).await
    else {
        return GeoSecuritySignal {
            score: base_score,
            decision: super::RiskDecision::Allow,
            factors,
            metadata: geo_metadata(None),
        };
    };

    let mut score = base_score;
    let mut geo_factors = Vec::new();
    if resolution.private_network {
        score += 10.0;
        geo_factors.push("private_or_special_ip");
    }
    if matches!(resolution.confidence.as_str(), "none" | "low") {
        score += 15.0;
        geo_factors.push("low_geo_confidence");
    }
    if resolution.source.as_str() == "fallback" {
        score += 10.0;
        geo_factors.push("geo_unresolved");
    }
    if resolution.risk_score >= 80 {
        score += 25.0;
        geo_factors.push("high_risk_network");
    } else if resolution.risk_score >= 60 {
        score += 12.0;
        geo_factors.push("elevated_risk_network");
    }
    let decision = geo_policy_decision(&resolution);
    if matches!(decision, super::RiskDecision::StepUp) {
        geo_factors.push("geo_policy_step_up");
    }

    if let Some(object) = factors.as_object_mut() {
        object.insert("geo_factors".to_string(), serde_json::json!(geo_factors));
        object.insert(
            "geo_country_code".to_string(),
            serde_json::json!(
                resolution
                    .location
                    .as_ref()
                    .map(|location| location.country_code.as_str())
            ),
        );
        object.insert(
            "geo_source".to_string(),
            serde_json::json!(resolution.source.as_str()),
        );
        object.insert(
            "geo_confidence".to_string(),
            serde_json::json!(resolution.confidence.as_str()),
        );
        object.insert(
            "geo_private_network".to_string(),
            serde_json::json!(resolution.private_network),
        );
        object.insert(
            "geo_network_kind".to_string(),
            serde_json::json!(resolution.network_kind.as_str()),
        );
        object.insert(
            "geo_risk_score".to_string(),
            serde_json::json!(resolution.risk_score),
        );
        object.insert(
            "geo_risk_labels".to_string(),
            serde_json::json!(resolution.risk_labels),
        );
    }

    GeoSecuritySignal {
        score: f64::min(score, 100.0),
        decision,
        factors,
        metadata: geo_metadata(Some(&resolution)),
    }
}

fn geo_policy_decision(resolution: &GeoResolution) -> super::RiskDecision {
    if resolution.private_network
        || matches!(resolution.confidence.as_str(), "none" | "low")
        || resolution.risk_score >= 60
        || matches!(
            resolution.network_kind,
            GeoNetworkKind::Tor
                | GeoNetworkKind::Proxy
                | GeoNetworkKind::Vpn
                | GeoNetworkKind::Datacenter
        )
    {
        return super::RiskDecision::StepUp;
    }
    super::RiskDecision::Allow
}

fn geo_metadata(resolution: Option<&GeoResolution>) -> Value {
    serde_json::json!({
        "geo_country_code": resolution.and_then(|resolution| {
            resolution
                .location
                .as_ref()
                .map(|location| location.country_code.as_str())
        }),
        "geo_source": resolution.map(|resolution| resolution.source.as_str()),
        "geo_confidence": resolution.map(|resolution| resolution.confidence.as_str()),
        "geo_network_kind": resolution.map(|resolution| resolution.network_kind.as_str()),
        "geo_risk_score": resolution.map(|resolution| resolution.risk_score),
        "geo_risk_labels": resolution.map(|resolution| &resolution.risk_labels),
    })
}

async fn resolve_and_record(
    db: &PgPool,
    config: &AppConfig,
    principal_id: Uuid,
    ip: Option<&str>,
    request_id: &'static str,
) -> Option<GeoResolution> {
    let mut tx = db.begin().await.ok()?;
    if let Err(error) =
        crate::domains::auth::geo_intelligence::ensure_ip_intelligence_tx(&mut tx, config, ip).await
    {
        tracing::warn!(%error, "auth ip intelligence cache refresh failed");
    }
    let resolution = match resolve_cached_geo_tx(&mut tx, ip, None, None).await {
        Ok(resolution) => resolution,
        Err(_) => {
            let _ = tx.rollback().await;
            return None;
        }
    };
    let _ = record_geo_resolution_tx(
        &mut tx,
        GeoLookupRecordContext {
            purpose: GeoLookupPurpose::Security,
            subject_type: Some("principal"),
            subject_id: Some(principal_id),
            request_id: Some(request_id),
        },
        &resolution,
    )
    .await;
    let _ = tx.commit().await;
    Some(resolution)
}
