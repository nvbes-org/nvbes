use serde::Serialize;

use crate::audit_evidence_center::AuditEvidenceSnapshot;

#[derive(Debug, Serialize)]
pub(crate) struct AuditEvidenceAlert {
    pub(crate) id: &'static str,
    pub(crate) severity: &'static str,
    pub(crate) title: &'static str,
    pub(crate) count: i64,
    pub(crate) target_anchor: &'static str,
}

#[derive(Debug, Serialize)]
pub(crate) struct RuntimeMetricAlert {
    pub(crate) id: &'static str,
    pub(crate) severity: &'static str,
    pub(crate) title: &'static str,
    pub(crate) metric_name: &'static str,
    pub(crate) condition: &'static str,
}

pub(crate) fn audit_evidence_alerts(snapshot: &AuditEvidenceSnapshot) -> Vec<AuditEvidenceAlert> {
    let mut alerts = Vec::new();
    push_alert(
        &mut alerts,
        snapshot.missing_hash_count,
        AuditEvidenceAlert {
            id: "missing_hash",
            severity: "critical",
            title: "Audit events without immutable hash",
            count: snapshot.missing_hash_count,
            target_anchor: "#audit-evidence-center",
        },
    );
    push_alert(
        &mut alerts,
        snapshot.backfilled_hash_count,
        AuditEvidenceAlert {
            id: "backfilled_hash",
            severity: "high",
            title: "Backfilled audit hashes require review",
            count: snapshot.backfilled_hash_count,
            target_anchor: "#audit-evidence-center",
        },
    );
    push_alert(
        &mut alerts,
        snapshot.actorless_event_count_24h,
        AuditEvidenceAlert {
            id: "actorless_events",
            severity: "high",
            title: "Audit events without actor in the last 24h",
            count: snapshot.actorless_event_count_24h,
            target_anchor: "#audit-evidence-center",
        },
    );
    if snapshot.active_signing_key_count == 0 {
        alerts.push(AuditEvidenceAlert {
            id: "no_active_signing_key",
            severity: "critical",
            title: "No active signing key for evidence chain",
            count: 1,
            target_anchor: "#audit-evidence-center",
        });
    }
    push_alert(
        &mut alerts,
        snapshot.deprecated_signing_key_count,
        AuditEvidenceAlert {
            id: "deprecated_signing_keys",
            severity: "medium",
            title: "Deprecated signing keys still visible",
            count: snapshot.deprecated_signing_key_count,
            target_anchor: "#audit-evidence-center",
        },
    );
    alerts
}

pub(crate) fn runtime_metric_alerts() -> Vec<RuntimeMetricAlert> {
    vec![
        RuntimeMetricAlert {
            id: "backoffice_mutation_failures",
            severity: "critical",
            title: "Back-office mutations are failing",
            metric_name: "internal_admin_action_requests_total",
            condition: "increase(metric{policy=~\"mutation|critical_mutation\",outcome=\"failed\"}[5m]) > 0",
        },
        RuntimeMetricAlert {
            id: "backoffice_guard_rejections",
            severity: "high",
            title: "Back-office guard rejections are increasing",
            metric_name: "internal_admin_guard_rejections_total",
            condition: "increase(metric[5m]) > 3",
        },
        RuntimeMetricAlert {
            id: "backoffice_rate_limited",
            severity: "high",
            title: "Back-office rate limits are being hit",
            metric_name: "internal_admin_rate_limited_total",
            condition: "increase(metric[5m]) > 0",
        },
    ]
}

fn push_alert(alerts: &mut Vec<AuditEvidenceAlert>, count: i64, alert: AuditEvidenceAlert) {
    if count > 0 {
        alerts.push(alert);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_evidence_alerts_surface_critical_evidence_gaps() {
        let alerts = audit_evidence_alerts(&AuditEvidenceSnapshot {
            audit_events_24h: 0,
            actorless_event_count_24h: 2,
            sensitive_action_count_24h: 0,
            missing_hash_count: 1,
            backfilled_hash_count: 0,
            linked_hash_count: 0,
            chain_head_count: 0,
            hash_anomaly_count: 1,
            active_signing_key_count: 0,
            deprecated_signing_key_count: 0,
            revoked_signing_key_count: 0,
            recent_audit_events: Vec::new(),
            actorless_events: Vec::new(),
            hash_anomalies: Vec::new(),
            signing_keys: Vec::new(),
            alerts: Vec::new(),
            runtime_alerts: Vec::new(),
        });

        let ids: Vec<&str> = alerts.iter().map(|alert| alert.id).collect();
        assert!(ids.contains(&"missing_hash"));
        assert!(ids.contains(&"actorless_events"));
        assert!(ids.contains(&"no_active_signing_key"));
    }

    #[test]
    fn runtime_alerts_include_mutation_failure_metric() {
        let alerts = runtime_metric_alerts();
        assert!(alerts.iter().any(|alert| {
            alert.id == "backoffice_mutation_failures"
                && alert.metric_name == "internal_admin_action_requests_total"
        }));
    }
}
