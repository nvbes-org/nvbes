use nvbes_region::geo::{
    GeoLookupRecordContext, GeoResolution, record_geo_resolution_tx, resolve_cached_geo_tx,
};
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domains::auth::risk;

pub(super) async fn record_login_geo_signal(
    db: &PgPool,
    principal_id: Uuid,
    ip: Option<&str>,
    client_country: Option<&str>,
    score: &mut f64,
    decision: &mut risk::RiskDecision,
    factors: &mut Value,
) -> Option<GeoResolution> {
    let resolution = resolve_and_record(db, principal_id, ip).await?;
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
    if geo_score_delta > 0.0 && matches!(decision, risk::RiskDecision::Allow) {
        *decision = risk::RiskDecision::StepUp;
    }
    *score = f64::min(*score + geo_score_delta, 100.0);

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
    }

    Some(resolution)
}

async fn resolve_and_record(
    db: &PgPool,
    principal_id: Uuid,
    ip: Option<&str>,
) -> Option<GeoResolution> {
    let mut tx = db.begin().await.ok()?;
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
            purpose: "auth",
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
