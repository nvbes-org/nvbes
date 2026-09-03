use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DegradedScenario {
    IdentityUnavailable,
    AccountUnavailable,
    StripeUnavailable,
    EmailDelayed,
    TrustRiskUnavailable,
    PostgresInRestore,
    FinOpsThresholdExceeded,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DegradedProcedure {
    pub scenario: DegradedScenario,
    pub title: String,
    pub severity: String,
    pub description: String,
    pub automatic_fallback: String,
    pub operator_checklist: Vec<String>,
    pub verification_steps: Vec<String>,
}

pub struct DegradedModeRegistry;

impl DegradedModeRegistry {
    pub fn get_procedure(scenario: DegradedScenario) -> DegradedProcedure {
        match scenario {
            DegradedScenario::IdentityUnavailable => DegradedProcedure {
                scenario,
                title: "Identity Service Unavailable".to_string(),
                severity: "SEV1".to_string(),
                description: "JWT token verification fails or Identity database is unreachable.".to_string(),
                automatic_fallback: "Maintain read-only for verified cached tokens; reject sensitive mutations with 503.".to_string(),
                operator_checklist: vec![
                    "Verify Scaleway serverless container status for identity-service.".to_string(),
                    "Check PostgreSQL connection pool for Identity database.".to_string(),
                    "Do NOT bypass JWT validation in downstream services.".to_string(),
                    "Engage Identity recovery runbook if container is crashlooping.".to_string(),
                ],
                verification_steps: vec![
                    "Verify /health/live and /health/ready on identity-service return 200.".to_string(),
                    "Test synthetic auth token generation.".to_string(),
                ],
            },
            DegradedScenario::AccountUnavailable => DegradedProcedure {
                scenario,
                title: "Account Service Unavailable".to_string(),
                severity: "SEV2".to_string(),
                description: "Profile updates, team memberships or privacy exports failing.".to_string(),
                automatic_fallback: "Queue pending mutations; return read-only profile state where cached.".to_string(),
                operator_checklist: vec![
                    "Inspect account-service error logs and metrics.".to_string(),
                    "Ensure ongoing GDPR export jobs are safely suspended without data loss.".to_string(),
                ],
                verification_steps: vec![
                    "Verify account-service health probes.".to_string(),
                    "Check team membership and profile read queries succeed.".to_string(),
                ],
            },
            DegradedScenario::StripeUnavailable => DegradedProcedure {
                scenario,
                title: "Stripe API / Webhooks Unavailable".to_string(),
                severity: "SEV2".to_string(),
                description: "Test Stripe API unreachable or webhook delivery failing.".to_string(),
                automatic_fallback: "Mark all pending checkouts as pending; NEVER cut off access automatically.".to_string(),
                operator_checklist: vec![
                    "Check Stripe status page and webhook signing secret.".to_string(),
                    "Verify webhook outbox in billing-service.".to_string(),
                    "Do NOT manually modify customer entitlements without logged audit.".to_string(),
                ],
                verification_steps: vec![
                    "Send signed test webhook event and verify processed status.".to_string(),
                ],
            },
            DegradedScenario::EmailDelayed => DegradedProcedure {
                scenario,
                title: "Email Delivery Delayed or Queued".to_string(),
                severity: "SEV2".to_string(),
                description: "Scaleway TEM or worker experiencing backlog or delivery retries.".to_string(),
                automatic_fallback: "Messages remain durably queued with exponential backoff; dead-letter after max attempts.".to_string(),
                operator_checklist: vec![
                    "Inspect email-worker queue depth and provider status.".to_string(),
                    "Verify suppressions list for false positives.".to_string(),
                ],
                verification_steps: vec![
                    "Trigger synthetic transactional email and monitor dispatch time.".to_string(),
                ],
            },
            DegradedScenario::TrustRiskUnavailable => DegradedProcedure {
                scenario,
                title: "Trust/Risk Service Unavailable".to_string(),
                severity: "SEV3".to_string(),
                description: "Trust/Risk gRPC evaluation endpoint unreachable.".to_string(),
                automatic_fallback: "Shadow mode default: allow operation, log signal for deferred batch evaluation.".to_string(),
                operator_checklist: vec![
                    "Verify trust-risk-service health probe.".to_string(),
                    "Confirm downstream services continue with shadow allow.".to_string(),
                ],
                verification_steps: vec![
                    "Verify trust_risk gRPC ping and review cases queue.".to_string(),
                ],
            },
            DegradedScenario::PostgresInRestore => DegradedProcedure {
                scenario,
                title: "PostgreSQL Database in Restoration".to_string(),
                severity: "SEV1".to_string(),
                description: "Restoration exercise or incident recovery in progress on database.".to_string(),
                automatic_fallback: "Application transitions to maintenance mode; freeze all write traffic.".to_string(),
                operator_checklist: vec![
                    "Confirm target is isolated non-production target or verified restore.".to_string(),
                    "Ensure RPO (<= 24h) and RTO (<= 8h) are tracked.".to_string(),
                    "Execute schema and integrity verification script.".to_string(),
                ],
                verification_steps: vec![
                    "Validate checksums, migration version, and record integrity.".to_string(),
                ],
            },
            DegradedScenario::FinOpsThresholdExceeded => DegradedProcedure {
                scenario,
                title: "FinOps Spend Threshold Exceeded".to_string(),
                severity: "SEV2".to_string(),
                description: "Monthly spend projected or confirmed to exceed budget stages.".to_string(),
                automatic_fallback: "Apply automatic stage guardrails (25 EUR disable non-essential, 28 EUR freeze cost creation, 30 EUR essential only).".to_string(),
                operator_checklist: vec![
                    "Check provider invoices and serverless container instance counts.".to_string(),
                    "Confirm min_scale is 0 on all services.".to_string(),
                    "Verify non-essential background jobs are stopped.".to_string(),
                ],
                verification_steps: vec![
                    "Run pnpm check:finops and verify spend stays strictly <= 30 EUR TTC.".to_string(),
                ],
            },
        }
    }

    pub fn all_procedures() -> Vec<DegradedProcedure> {
        vec![
            Self::get_procedure(DegradedScenario::IdentityUnavailable),
            Self::get_procedure(DegradedScenario::AccountUnavailable),
            Self::get_procedure(DegradedScenario::StripeUnavailable),
            Self::get_procedure(DegradedScenario::EmailDelayed),
            Self::get_procedure(DegradedScenario::TrustRiskUnavailable),
            Self::get_procedure(DegradedScenario::PostgresInRestore),
            Self::get_procedure(DegradedScenario::FinOpsThresholdExceeded),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provides_all_seven_mandatory_procedures() {
        let procedures = DegradedModeRegistry::all_procedures();
        assert_eq!(procedures.len(), 7);
        for p in &procedures {
            assert!(!p.operator_checklist.is_empty());
            assert!(!p.verification_steps.is_empty());
        }
    }
}
