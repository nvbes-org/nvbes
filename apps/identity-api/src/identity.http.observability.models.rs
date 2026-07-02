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
                    id: "security-billing",
                    name: "Billing Protection",
                    purpose: "Track checkout, portal access, billing abuse blocks, and webhook health.",
                    signals: vec![
                        "billing_checkout_started",
                        "billing_portal_opened",
                        "billing_operation_blocked",
                        "billing_webhook_received",
                        "billing_webhook_replayed",
                        "billing_webhook_failed",
                    ],
                },
                DashboardDefinition {
                    id: "billing-worker",
                    name: "Billing Worker",
                    purpose: "Track billing provider queue depth, stale jobs, retries, and dead-letter pressure.",
                    signals: vec![
                        "worker_queue_depth",
                        "worker_queue_oldest_age_seconds",
                        "worker_queue_jobs_total",
                        "worker_queue_recovered_jobs_total",
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
                AlertDefinition {
                    id: "billing-abuse",
                    name: "Billing abuse",
                    severity: "high",
                    condition: "billing_operation_blocked or repeated portal/checkout throttling",
                },
                AlertDefinition {
                    id: "billing-webhook-failure",
                    name: "Billing webhook failure",
                    severity: "critical",
                    condition: "billing_webhook_failed spikes or persistently fails",
                },
                AlertDefinition {
                    id: "billing-webhook-replay",
                    name: "Billing webhook replay",
                    severity: "high",
                    condition: "billing_webhook_replayed rate increases unexpectedly",
                },
                AlertDefinition {
                    id: "billing-worker-backlog",
                    name: "Billing worker backlog",
                    severity: "high",
                    condition: "worker_queue_depth grows or oldest queued job age exceeds threshold",
                },
                AlertDefinition {
                    id: "billing-worker-dead-letter",
                    name: "Billing worker dead-letter",
                    severity: "critical",
                    condition: "worker_queue_jobs_total outcome = dead_letter increases",
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
                LogStreamDefinition {
                    id: "billing-guard",
                    name: "Billing Guard Stream",
                    signals: vec![
                        "billing_operation_blocked",
                        "billing_checkout_started",
                        "billing_webhook_received",
                        "billing_webhook_replayed",
                        "billing_webhook_failed",
                    ],
                },
                LogStreamDefinition {
                    id: "billing-worker",
                    name: "Billing Worker Stream",
                    signals: vec![
                        "worker_queue_depth",
                        "worker_queue_oldest_age_seconds",
                        "worker_queue_jobs_total",
                        "worker_queue_recovered_jobs_total",
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
        assert!(
            alerts
                .alerts
                .iter()
                .any(|alert| alert.id == "billing-webhook-failure")
        );
        assert!(
            alerts
                .alerts
                .iter()
                .any(|alert| alert.id == "billing-webhook-replay")
        );
        assert!(
            alerts
                .alerts
                .iter()
                .any(|alert| alert.id == "billing-worker-backlog")
        );
        assert!(
            alerts
                .alerts
                .iter()
                .any(|alert| alert.id == "billing-worker-dead-letter")
        );
        assert!(streams.streams.iter().any(|stream| {
            stream
                .signals
                .contains(&"enterprise_recovery_review_pending")
        }));
        assert!(
            streams
                .streams
                .iter()
                .any(|stream| stream.signals.contains(&"billing_webhook_failed"))
        );
        assert!(
            dashboards
                .dashboards
                .iter()
                .any(|dashboard| dashboard.id == "billing-worker")
        );
        assert!(
            streams
                .streams
                .iter()
                .any(|stream| stream.id == "billing-worker")
        );
    }
}
