use crate::cockpit_model::ServiceId;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaseCategory {
    Support,
    Security,
    Abuse,
    Billing,
    Appeal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaseStatus {
    Open,
    Investigating,
    AwaitingUser,
    ActionPending,
    Resolved,
    Appealed,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Case {
    pub id: Uuid,
    pub version: i64,
    pub category: CaseCategory,
    pub owner: ServiceId,
    pub subject_id: Uuid,
    pub source: String,
    pub summary: String,
    pub status: CaseStatus,
    pub related_case_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Command {
    pub idempotency_key: Uuid,
    pub correlation_id: Uuid,
    pub reason: String,
    pub action: Action,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    OpenCase {
        category: CaseCategory,
        owner: ServiceId,
        subject_id: Uuid,
        source: String,
        summary: String,
        related_case_id: Option<Uuid>,
    },
    Transition {
        case_id: Uuid,
        expected_version: i64,
        status: CaseStatus,
        evidence: String,
    },
    AddNote {
        case_id: Uuid,
        expected_version: i64,
        note: String,
        evidence: String,
    },
    RecordObservation {
        case_id: Uuid,
        expected_version: i64,
        service: ServiceId,
        observed_at: DateTime<Utc>,
        api_reference: String,
        summary: String,
    },
    RecordCost {
        month: NaiveDate,
        provider: String,
        category: String,
        actual_cents: u32,
        forecast_cents: u32,
        evidence: String,
        replaces: Option<Uuid>,
    },
}

impl Action {
    pub fn case_id(&self) -> Option<Uuid> {
        match self {
            Self::Transition { case_id, .. }
            | Self::AddNote { case_id, .. }
            | Self::RecordObservation { case_id, .. } => Some(*case_id),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Receipt {
    pub id: Uuid,
    pub actor: String,
    pub correlation_id: Uuid,
    pub case_id: Option<Uuid>,
    pub version: Option<i64>,
    pub cost_id: Option<Uuid>,
    pub executed_at: DateTime<Utc>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cost {
    pub id: Uuid,
    pub month: NaiveDate,
    pub provider: String,
    pub category: String,
    pub actual_cents: u32,
    pub forecast_cents: u32,
    pub evidence: String,
    pub replaces: Option<Uuid>,
}
