use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub struct DashboardsResponse {
    pub dashboards: Vec<DashboardDefinition>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub struct DashboardDefinition {
    pub id: &'static str,
    pub title: &'static str,
    pub panels: Vec<&'static str>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub struct CriticalAlertsResponse {
    pub alerts: Vec<CriticalAlertDefinition>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub struct CriticalAlertDefinition {
    pub id: &'static str,
    pub severity: &'static str,
    pub signal: &'static str,
    pub condition: &'static str,
    pub owner: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub struct LogStreamsResponse {
    pub streams: Vec<LogStreamDefinition>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub struct LogStreamDefinition {
    pub id: &'static str,
    pub purpose: &'static str,
    pub allowed_payload: Vec<&'static str>,
    pub forbidden_payload: Vec<&'static str>,
}

impl DashboardsResponse {
    pub fn v1() -> Self {
        Self {
            dashboards: vec![
                DashboardDefinition {
                    id: "api-health",
                    title: "API health",
                    panels: vec![
                        "requests_total_by_status",
                        "p95_latency_ms",
                        "5xx_rate",
                        "active_requests",
                    ],
                },
                DashboardDefinition {
                    id: "uploads-downloads",
                    title: "Uploads and downloads",
                    panels: vec![
                        "upload_success_failure",
                        "download_success_failure",
                        "upload_duration",
                        "object_storage_errors",
                    ],
                },
                DashboardDefinition {
                    id: "workers-jobs",
                    title: "Workers and jobs",
                    panels: vec!["queue_depth", "jobs_failed", "jobs_retried", "dead_letters"],
                },
                DashboardDefinition {
                    id: "security",
                    title: "Security events",
                    panels: vec![
                        "auth_failures",
                        "permission_denied",
                        "api_key_denied",
                        "public_api_geo_requests_by_network_kind",
                        "public_api_high_risk_requests",
                        "public_api_network_policy_blocks_by_reason",
                        "audit_events_created",
                    ],
                },
                DashboardDefinition {
                    id: "finops",
                    title: "FinOps",
                    panels: vec![
                        "storage_cost_by_workspace",
                        "egress_by_workspace",
                        "logs_volume_by_environment",
                        "gross_margin_by_plan",
                    ],
                },
            ],
        }
    }
}

impl CriticalAlertsResponse {
    pub fn v1() -> Self {
        Self {
            alerts: vec![
                CriticalAlertDefinition {
                    id: "api-5xx-high",
                    severity: "critical",
                    signal: "technical_log",
                    condition: "5xx rate above 2% for 5 minutes",
                    owner: "engineering",
                },
                CriticalAlertDefinition {
                    id: "uploads-failing",
                    severity: "critical",
                    signal: "metrics",
                    condition: "upload failure rate above 5% for 10 minutes",
                    owner: "engineering",
                },
                CriticalAlertDefinition {
                    id: "worker-queue-stalled",
                    severity: "critical",
                    signal: "metrics",
                    condition: "queue depth grows for 15 minutes or oldest job age above 30 minutes",
                    owner: "engineering",
                },
                CriticalAlertDefinition {
                    id: "postgres-saturation",
                    severity: "critical",
                    signal: "metrics",
                    condition: "PostgreSQL unavailable or pool saturation above 90%",
                    owner: "engineering",
                },
                CriticalAlertDefinition {
                    id: "object-storage-unavailable",
                    severity: "critical",
                    signal: "metrics",
                    condition: "object storage errors above threshold for 5 minutes",
                    owner: "engineering",
                },
                CriticalAlertDefinition {
                    id: "budget-100",
                    severity: "critical",
                    signal: "finops",
                    condition: "monthly cloud budget reaches 100%",
                    owner: "ops-finance",
                },
                CriticalAlertDefinition {
                    id: "public-bucket-suspected",
                    severity: "critical",
                    signal: "security",
                    condition: "bucket policy scan detects public access",
                    owner: "security",
                },
                CriticalAlertDefinition {
                    id: "public-api-network-risk-spike",
                    severity: "high",
                    signal: "security",
                    condition: "drive_public_api_network_policy_blocks_total increases unexpectedly",
                    owner: "security",
                },
            ],
        }
    }
}

impl LogStreamsResponse {
    pub fn v1() -> Self {
        Self {
            streams: vec![
                LogStreamDefinition {
                    id: "technical_log",
                    purpose: "Operate the platform and debug runtime failures.",
                    allowed_payload: vec![
                        "request_id",
                        "method",
                        "path_template",
                        "status",
                        "duration_ms",
                        "service",
                        "environment",
                    ],
                    forbidden_payload: vec!["api_keys", "tokens", "signed_urls", "file_content"],
                },
                LogStreamDefinition {
                    id: "audit",
                    purpose: "Append-only product and security evidence for owner/admin review.",
                    allowed_payload: vec![
                        "workspace_id",
                        "actor_id",
                        "action",
                        "target_type",
                        "target_id",
                        "request_id",
                        "geo_country_code",
                        "geo_network_kind",
                        "geo_risk_score",
                        "geo_risk_labels",
                        "network_block_reason",
                    ],
                    forbidden_payload: vec!["api_key_secret", "raw_password", "file_content"],
                },
                LogStreamDefinition {
                    id: "analytics",
                    purpose: "Product usage measurement with consent-aware event taxonomy.",
                    allowed_payload: vec![
                        "workspace_id",
                        "plan_code",
                        "event_name",
                        "cohort",
                        "source",
                    ],
                    forbidden_payload: vec![
                        "file_names",
                        "emails",
                        "api_keys",
                        "tokens",
                        "object_keys",
                        "personal_content",
                    ],
                },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::domains::public_api::metrics::{
        GEO_REQUESTS_METRIC, HIGH_RISK_REQUESTS_METRIC, NETWORK_POLICY_BLOCKS_METRIC,
    };

    use super::{CriticalAlertsResponse, DashboardsResponse, LogStreamsResponse};

    #[test]
    fn observability_contract_includes_public_api_network_risk() {
        let dashboards = DashboardsResponse::v1();
        let alerts = CriticalAlertsResponse::v1();
        let streams = LogStreamsResponse::v1();

        assert!(dashboards.dashboards.iter().any(|dashboard| {
            dashboard
                .panels
                .contains(&"public_api_network_policy_blocks_by_reason")
        }));
        assert!(
            alerts
                .alerts
                .iter()
                .any(|alert| alert.id == "public-api-network-risk-spike"
                    && alert.condition.contains(NETWORK_POLICY_BLOCKS_METRIC))
        );
        assert!(GEO_REQUESTS_METRIC.starts_with("drive_public_api_"));
        assert!(HIGH_RISK_REQUESTS_METRIC.starts_with("drive_public_api_"));
        assert!(streams.streams.iter().any(|stream| {
            stream.id == "audit"
                && stream.allowed_payload.contains(&"geo_network_kind")
                && stream.allowed_payload.contains(&"network_block_reason")
        }));
    }
}
