use std::collections::BTreeMap;

use crate::domains::security::types::{RiskEventView, SecurityEventCount, SecurityEventsSummary};

pub(super) fn security_events_summary(events: &[RiskEventView]) -> SecurityEventsSummary {
    let mut by_network_kind = BTreeMap::new();
    let mut by_country = BTreeMap::new();
    let mut by_risk_label = BTreeMap::new();
    let mut high_risk_events = 0;

    for event in events {
        if event.geo_risk_score.unwrap_or_default() >= 80 || event.risk_score >= 80.0 {
            high_risk_events += 1;
        }
        increment_optional(&mut by_network_kind, event.geo_network_kind.as_deref());
        increment_optional(&mut by_country, event.geo_country_code.as_deref());
        for label in &event.geo_risk_labels {
            increment(&mut by_risk_label, label);
        }
    }

    SecurityEventsSummary {
        total_events: events.len(),
        high_risk_events,
        by_network_kind: top_counts(by_network_kind),
        by_country: top_counts(by_country),
        by_risk_label: top_counts(by_risk_label),
    }
}

fn increment_optional(counts: &mut BTreeMap<String, usize>, key: Option<&str>) {
    if let Some(key) = key.map(str::trim).filter(|key| !key.is_empty()) {
        increment(counts, key);
    }
}

fn increment(counts: &mut BTreeMap<String, usize>, key: &str) {
    *counts.entry(key.to_string()).or_default() += 1;
}

fn top_counts(counts: BTreeMap<String, usize>) -> Vec<SecurityEventCount> {
    let mut counts = counts
        .into_iter()
        .map(|(key, count)| SecurityEventCount { key, count })
        .collect::<Vec<_>>();
    counts.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.key.cmp(&right.key))
    });
    counts.truncate(10);
    counts
}

#[cfg(test)]
mod tests {
    use super::security_events_summary;
    use crate::domains::security::types::RiskEventView;
    use chrono::{TimeZone, Utc};
    use serde_json::json;
    use uuid::Uuid;

    #[test]
    fn summarizes_network_country_and_labels() {
        let events = vec![
            event(
                Some("vpn"),
                Some("FR"),
                Some(90),
                vec!["vpn".to_string(), "datacenter".to_string()],
                20.0,
            ),
            event(
                Some("vpn"),
                Some("FR"),
                Some(90),
                vec!["vpn".to_string()],
                80.0,
            ),
            event(
                Some("residential"),
                Some("DE"),
                Some(15),
                vec!["residential".to_string()],
                15.0,
            ),
        ];

        let summary = security_events_summary(&events);

        assert_eq!(summary.total_events, 3);
        assert_eq!(summary.high_risk_events, 2);
        assert_eq!(summary.by_network_kind[0].key, "vpn");
        assert_eq!(summary.by_network_kind[0].count, 2);
        assert_eq!(summary.by_country[0].key, "FR");
        assert_eq!(summary.by_risk_label[0].key, "vpn");
    }

    fn event(
        kind: Option<&str>,
        country: Option<&str>,
        geo_score: Option<i64>,
        labels: Vec<String>,
        risk_score: f64,
    ) -> RiskEventView {
        RiskEventView {
            id: Uuid::new_v4(),
            principal_id: Uuid::new_v4(),
            session_id: None,
            device_id: None,
            event_type: "login_success".to_string(),
            ip_address: Some("203.0.113.42".to_string()),
            user_agent: None,
            risk_score,
            risk_factors: json!({}),
            decision: "allow".to_string(),
            metadata: json!({}),
            geo_country_code: country.map(ToOwned::to_owned),
            geo_source: Some("remote_lookup".to_string()),
            geo_confidence: Some("medium".to_string()),
            geo_network_kind: kind.map(ToOwned::to_owned),
            geo_risk_score: geo_score,
            geo_risk_labels: labels,
            created_at: Utc
                .with_ymd_and_hms(2026, 6, 23, 12, 0, 0)
                .single()
                .expect("valid timestamp"),
        }
    }
}
