use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub struct DashboardsResponse {
    pub version: &'static str,
    pub dashboards: Vec<DashboardDefinition>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub struct CriticalAlertsResponse {
    pub version: &'static str,
    pub alerts: Vec<AlertDefinition>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub struct LogStreamsResponse {
    pub version: &'static str,
    pub streams: Vec<LogStreamDefinition>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub struct DashboardDefinition {
    pub id: &'static str,
    pub name: &'static str,
    pub purpose: &'static str,
    pub signals: Vec<&'static str>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub struct AlertDefinition {
    pub id: &'static str,
    pub name: &'static str,
    pub severity: &'static str,
    pub condition: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub struct LogStreamDefinition {
    pub id: &'static str,
    pub name: &'static str,
    pub signals: Vec<&'static str>,
}

impl DashboardsResponse {
    pub fn v1() -> Self {
        Self {
            version: "v1",
            dashboards: vec![
                DashboardDefinition {
                    id: "security-auth",
                    name: "Authentication Security",
                    purpose: "Follow login, step-up, MFA, recovery, and lockout pressure.",
                    signals: vec![
                        "login_failed",
                        "password_reset_requested",
                        "enterprise_recovery_requested",
                        "enterprise_recovery_review_pending",
                        "enterprise_recovery_approved",
                        "webauthn_failed",
                        "risk_policy_blocked",
                    ],
                },
                DashboardDefinition {
                    id: "security-rate-limits",
                    name: "Rate Limit Pressure",
                    purpose: "Monitor shared throttling across replicas and public entrypoints.",
                    signals: vec![
                        "auth_forgot_password",
                        "auth_reset_password",
                        "oauth_authorize",
                        "oauth_token",
                        "device_authorize",
                    ],
                },
            ],
        }
    }
}

impl CriticalAlertsResponse {
    pub fn v1() -> Self {
        Self {
            version: "v1",
            alerts: vec![
                AlertDefinition {
                    id: "auth-lockout-spike",
                    name: "Auth lockout spike",
                    severity: "critical",
                    condition: "risk_events.decision = lock over 5 minutes",
                },
                AlertDefinition {
                    id: "recovery-bypass-attempt",
                    name: "Recovery bypass attempt",
                    severity: "critical",
                    condition: "enterprise recovery approval before cooldown expiry",
                },
                AlertDefinition {
                    id: "recovery-review-backlog",
                    name: "Recovery review backlog",
                    severity: "high",
                    condition: "enterprise_recovery_review_pending older than review window",
                },
            ],
        }
    }
}

impl LogStreamsResponse {
    pub fn v1() -> Self {
        Self {
            version: "v1",
            streams: vec![
                LogStreamDefinition {
                    id: "auth-risk",
                    name: "Auth Risk Stream",
                    signals: vec!["risk_events", "login_failed", "password_reset_requested"],
                },
                LogStreamDefinition {
                    id: "recovery-flow",
                    name: "Recovery Flow Stream",
                    signals: vec![
                        "enterprise_recovery_requested",
                        "enterprise_recovery_review_pending",
                        "enterprise_recovery_approved",
                        "password_reset_completed",
                    ],
                },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CriticalAlertsResponse, DashboardsResponse, LogStreamsResponse};

    #[test]
    fn observability_contract_includes_enterprise_recovery_review_pending() {
        let dashboards = DashboardsResponse::v1();
        let alerts = CriticalAlertsResponse::v1();
        let streams = LogStreamsResponse::v1();

        assert!(dashboards.dashboards.iter().any(|dashboard| {
            dashboard
                .signals
                .contains(&"enterprise_recovery_review_pending")
        }));
        assert!(
            alerts
                .alerts
                .iter()
                .any(|alert| alert.id == "recovery-review-backlog")
        );
        assert!(streams.streams.iter().any(|stream| {
            stream
                .signals
                .contains(&"enterprise_recovery_review_pending")
        }));
        let webhook_alert_fragment = ["billing", "webhook"].join("-");
        assert!(
            !alerts
                .alerts
                .iter()
                .any(|alert| alert.id.contains(&webhook_alert_fragment))
        );
        let webhook_signal_fragment = ["billing", "webhook"].join("_");
        assert!(!streams.streams.iter().any(|stream| {
            stream
                .signals
                .iter()
                .any(|signal| signal.contains(&webhook_signal_fragment))
        }));
        assert!(
            dashboards.dashboards.iter().all(|dashboard| {
                !dashboard.id.contains("billing")
                    && dashboard
                        .signals
                        .iter()
                        .all(|signal| !signal.starts_with("billing"))
            }),
            "Identity observability dashboards must not expose Billing runtime signals"
        );
        assert!(
            alerts.alerts.iter().all(|alert| {
                !alert.id.contains("billing") && !alert.condition.contains("billing")
            }),
            "Identity observability alerts must not expose Billing runtime signals"
        );
        assert!(
            streams.streams.iter().all(|stream| {
                !stream.id.contains("billing")
                    && stream
                        .signals
                        .iter()
                        .all(|signal| !signal.starts_with("billing"))
            }),
            "Identity observability streams must not expose Billing runtime signals"
        );
    }
}
