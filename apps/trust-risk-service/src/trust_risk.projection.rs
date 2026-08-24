use std::{collections::BTreeSet, time::Duration};

use chrono::{DateTime, Utc};
use nvbes_trust_risk::{attribute::Attribute, rules::FeatureMap, signal::RiskSignal};
use tokio::sync::watch;
use uuid::Uuid;

use crate::{app::TrustRiskState, projection_db};

pub const FEATURE_VERSION: &str = "features-v1";
pub const MAX_PROJECTION_ATTEMPTS: u32 = 8;
pub const ALLOWED_LATENESS: chrono::Duration = chrono::Duration::minutes(5);

pub fn derive_features<'a>(
    signals: impl IntoIterator<Item = &'a RiskSignal>,
    watermark: DateTime<Utc>,
) -> FeatureMap {
    let mut features = FeatureMap::new();
    let mut linked_principals = BTreeSet::new();
    let mut linked_tenants = BTreeSet::new();
    let mut events_1h = 0_u64;
    let mut events_24h = 0_u64;
    let mut high_risk_events_1h = 0_u64;
    let mut automation_confidence = 0.0_f64;

    for signal in signals {
        let age = watermark.signed_duration_since(signal.occurred_at());
        if age < chrono::Duration::zero() || age > chrono::Duration::hours(24) {
            continue;
        }
        events_24h += 1;
        for subject in signal.subjects() {
            match subject.kind() {
                nvbes_trust_risk::proto::nvbes::trust_risk::v1::SubjectKind::Principal => {
                    linked_principals.insert((subject.namespace(), subject.opaque_id()));
                }
                nvbes_trust_risk::proto::nvbes::trust_risk::v1::SubjectKind::Tenant => {
                    linked_tenants.insert((subject.namespace(), subject.opaque_id()));
                }
                _ => {}
            }
        }
        if age <= chrono::Duration::hours(1) {
            events_1h += 1;
            let risk_score = numeric(signal.attributes().get("risk_score"))
                .or_else(|| numeric(signal.attributes().get("provider_risk_score")))
                .unwrap_or_default();
            if risk_score >= 75.0 {
                high_risk_events_1h += 1;
            }
            if signal.kind().starts_with("network.") {
                set_max(&mut features, "network_risk_score_max_1h", risk_score);
            }
            if signal.kind().starts_with("automation.") {
                automation_confidence = automation_confidence
                    .max(numeric(signal.attributes().get("confidence")).unwrap_or_default());
            }
        }
    }
    features.insert("events_1h".to_string(), events_1h as f64);
    features.insert("events_24h".to_string(), events_24h as f64);
    features.insert(
        "high_risk_events_1h".to_string(),
        high_risk_events_1h as f64,
    );
    features.insert(
        "linked_principals_24h".to_string(),
        linked_principals.len() as f64,
    );
    features.insert(
        "linked_tenants_24h".to_string(),
        linked_tenants.len() as f64,
    );
    features.insert(
        "automation_confidence_max_1h".to_string(),
        automation_confidence,
    );
    features.entry("negative_labels".to_string()).or_default();
    features.entry("positive_labels".to_string()).or_default();
    features
}

pub fn is_too_late(occurred_at: DateTime<Utc>, watermark: DateTime<Utc>) -> bool {
    occurred_at < watermark - chrono::Duration::hours(24) - ALLOWED_LATENESS
}

pub fn retry_delay(attempt: u32, signal_id: Uuid) -> Option<Duration> {
    if attempt >= MAX_PROJECTION_ATTEMPTS {
        return None;
    }
    let exponent = attempt.saturating_sub(1).min(6);
    let base = 1_u64 << exponent;
    let jitter = u64::from(signal_id.as_bytes()[0]) % (base + 1);
    Some(Duration::from_secs((base + jitter).min(60)))
}

pub async fn run(state: TrustRiskState, mut shutdown: watch::Receiver<bool>) {
    let worker_id = Uuid::new_v4();
    loop {
        if *shutdown.borrow() {
            break;
        }
        match projection_db::process_batch(&state.db, worker_id, 64).await {
            Ok(_) => *state.projection_heartbeat.write().await = Some(std::time::Instant::now()),
            Err(error) => tracing::warn!(error = %error, "trust/risk projection cycle failed"),
        }
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_millis(250)) => {}
            result = shutdown.changed() => if result.is_err() { break; },
        }
    }
}

fn numeric(value: Option<&Attribute>) -> Option<f64> {
    match value {
        Some(Attribute::Signed(value)) => Some(*value as f64),
        Some(Attribute::Unsigned(value)) => Some(*value as f64),
        Some(Attribute::Decimal(value)) => Some(*value),
        _ => None,
    }
}

fn set_max(features: &mut FeatureMap, name: &str, candidate: f64) {
    let value = features.entry(name.to_string()).or_default();
    *value = value.max(candidate);
}

#[cfg(test)]
#[path = "trust_risk.projection.tests.rs"]
mod tests;
