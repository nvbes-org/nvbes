use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Default)]
pub struct AccessReviewScheduleRun {
    pub campaigns_created: u64,
    pub empty_schedules: u64,
}

#[derive(Debug, Default)]
pub struct AccessReviewReminderRun {
    pub reminders_enqueued: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct AccessReviewCampaignScopeInput {
    #[serde(default = "default_true")]
    pub include_members: bool,
    #[serde(default = "default_true")]
    pub include_roles: bool,
    #[serde(default = "default_true")]
    pub include_service_accounts: bool,
    #[serde(default = "default_true")]
    pub include_oauth_clients: bool,
}

pub(crate) enum ScheduleExecution {
    Created {
        campaign_id: Uuid,
        due_at: DateTime<Utc>,
        item_count: u64,
    },
    Empty,
}

#[derive(Debug, FromRow)]
pub(crate) struct ScheduleExecutionRow {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub include_members: bool,
    pub include_roles: bool,
    pub include_service_accounts: bool,
    pub include_oauth_clients: bool,
    pub recurrence_days: i32,
    pub due_after_days: i32,
    pub created_by: Uuid,
    pub disabled_at: Option<DateTime<Utc>>,
}

impl ScheduleExecutionRow {
    pub(crate) fn scope(&self) -> AccessReviewCampaignScopeInput {
        AccessReviewCampaignScopeInput {
            include_members: self.include_members,
            include_roles: self.include_roles,
            include_service_accounts: self.include_service_accounts,
            include_oauth_clients: self.include_oauth_clients,
        }
    }
}

#[derive(Debug, FromRow)]
pub(crate) struct AccessReviewReminderCandidate {
    pub tenant_id: Uuid,
    pub campaign_id: Uuid,
    pub campaign_name: String,
    pub tenant_name: String,
    pub due_at: DateTime<Utc>,
    pub pending_items: i64,
    pub recipient_principal_id: Uuid,
    pub recipient_email: String,
    pub recipient_name: String,
    pub reminder_kind: String,
}

fn default_true() -> bool {
    true
}
