use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::finops_budget::BudgetStage;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unavailable,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ServiceId {
    Identity,
    Account,
    Billing,
    Email,
    TrustRisk,
}

impl ServiceId {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Identity => "identity-service",
            Self::Account => "account-service",
            Self::Billing => "billing-service",
            Self::Email => "email-worker",
            Self::TrustRisk => "trust-risk-service",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeHealth {
    pub service_id: ServiceId,
    pub status: HealthStatus,
    pub live: bool,
    pub ready: bool,
    pub latency_ms: u64,
    pub is_cold_start: bool,
    pub details: String,
    pub checked_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AggregateHealth {
    pub overall_status: HealthStatus,
    pub runtimes: Vec<RuntimeHealth>,
    pub degraded_count: usize,
    pub evaluated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OutboxJobsSummary {
    pub pending_count: u64,
    pub failed_count: u64,
    pub dead_letter_count: u64,
    pub unprocessed_events_count: u64,
    pub last_processed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrustRiskSummary {
    pub shadow_mode: bool,
    pub pending_reviews_count: u64,
    pub allow_count: u64,
    pub challenge_count: u64,
    pub review_count: u64,
    pub deny_count: u64,
    pub active_cases: Vec<TrustRiskCaseItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrustRiskCaseItem {
    pub case_id: Uuid,
    pub evaluation_id: Uuid,
    pub score: i16,
    pub band: String,
    pub recommendation: String,
    pub state: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EmailOperationsSummary {
    pub queued_count: u64,
    pub sent_count_24h: u64,
    pub delivered_count_24h: u64,
    pub failed_count_24h: u64,
    pub bounce_count_24h: u64,
    pub active_suppressions_count: u64,
    pub unprocessed_events_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BillingReconciliationSummary {
    pub test_mode: bool,
    pub webhook_events_24h: u64,
    pub unverified_webhooks_count: u64,
    pub pending_reconciliations_count: u64,
    pub reconciliation_mismatch_count: u64,
    pub last_reconciliation_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FinOpsCategorySpend {
    pub category: String,
    pub current_cents: u32,
    pub target_cents: u32,
    pub limit_cents: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FinOpsBudgetSummary {
    pub current_spend_cents: u32,
    pub target_cents: u32,
    pub hard_limit_cents: u32,
    pub stage: BudgetStage,
    pub projected_monthly_cents: u32,
    pub categories: Vec<FinOpsCategorySpend>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackupRestoreSummary {
    pub latest_backup_at: Option<DateTime<Utc>>,
    pub rpo_target_hours: u32,
    pub rto_target_hours: u32,
    pub last_restore_verified_at: Option<DateTime<Utc>>,
    pub restore_verification_passed: bool,
    pub meets_rpo: bool,
    pub meets_rto: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CockpitOverview {
    pub environment: String,
    pub generated_at: DateTime<Utc>,
    pub health: AggregateHealth,
    pub outbox_jobs: OutboxJobsSummary,
    pub trust_risk: TrustRiskSummary,
    pub email: EmailOperationsSummary,
    pub billing: BillingReconciliationSummary,
    pub finops: FinOpsBudgetSummary,
    pub backup_restore: BackupRestoreSummary,
    pub degraded_procedures_active: usize,
}
