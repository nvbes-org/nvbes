use nvbes_core::config::AppConfig;
use nvbes_region::geo::{
    GeoLookupPurpose, GeoLookupRecordContext, GeoNetworkKind, GeoResolution,
    record_geo_resolution_tx, resolve_cached_geo_tx,
};
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domains::auth::risk;

pub(super) async fn record_login_geo_signal(
    db: &PgPool,
    config: &AppConfig,
    principal_id: Uuid,
    ip: Option<&str>,
    client_country: Option<&str>,
    score: &mut f64,
    decision: &mut risk::RiskDecision,
    factors: &mut Value,
) -> Option<GeoResolution> {
    let resolution = resolve_and_record(db, config, principal_id, ip).await?;
    let mut geo_score_delta = 0.0;
    let mut geo_factors = Vec::new();

    if resolution.private_network {
        geo_score_delta += 10.0;
        geo_factors.push("private_or_special_ip");
    }
    if matches!(resolution.confidence.as_str(), "none" | "low") {
        geo_score_delta += 15.0;
        geo_factors.push("low_geo_confidence");
    }
    if resolution.source.as_str() == "fallback" {
        geo_score_delta += 10.0;
        geo_factors.push("geo_unresolved");
    }
    if resolution.risk_score >= 80 {
        geo_score_delta += 25.0;
        geo_factors.push("high_risk_network");
    } else if resolution.risk_score >= 60 {
        geo_score_delta += 12.0;
        geo_factors.push("elevated_risk_network");
    }
    let geo_decision = geo_policy_decision(&resolution);
    if matches!(geo_decision, risk::RiskDecision::StepUp) {
        geo_factors.push("geo_policy_step_up");
    }
    let resolved_country = resolution
        .location
        .as_ref()
        .map(|location| location.country_code.as_str());
    if let (Some(resolved), Some(client_country)) = (resolved_country, client_country)
        && resolved != client_country
    {
        geo_score_delta += 20.0;
        geo_factors.push("geo_client_country_mismatch");
    }
    *score = f64::min(*score + geo_score_delta, 100.0);
    *decision = (*decision).strictest(geo_decision);

    if let Some(object) = factors.as_object_mut() {
        object.insert(
            "geo_score_delta".to_string(),
            serde_json::json!(geo_score_delta),
        );
        object.insert("geo_factors".to_string(), serde_json::json!(geo_factors));
        object.insert(
            "geo_country_code".to_string(),
            serde_json::json!(resolved_country),
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

    Some(resolution)
}

fn geo_policy_decision(resolution: &GeoResolution) -> risk::RiskDecision {
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
        return risk::RiskDecision::StepUp;
    }
    risk::RiskDecision::Allow
}

async fn resolve_and_record(
    db: &PgPool,
    config: &AppConfig,
    principal_id: Uuid,
    ip: Option<&str>,
) -> Option<GeoResolution> {
    let mut tx = db.begin().await.ok()?;
    if let Err(error) =
        crate::domains::auth::geo_intelligence::ensure_ip_intelligence_tx(&mut tx, config, ip).await
    {
        tracing::warn!(%error, "login ip intelligence cache refresh failed");
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
            purpose: GeoLookupPurpose::Auth,
            subject_type: Some("principal"),
            subject_id: Some(principal_id),
            request_id: None,
        },
        &resolution,
    )
    .await;
    let _ = tx.commit().await;
    Some(resolution)
}
