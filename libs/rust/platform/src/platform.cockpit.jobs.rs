use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::cockpit_model::OutboxJobsSummary;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Pending,
    Processing,
    Completed,
    Failed,
    DeadLetter,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FailedEventRecord {
    pub event_id: Uuid,
    pub event_type: String,
    pub correlation_id: Uuid,
    pub attempt_count: u32,
    pub last_error: String,
    pub state: JobState,
    pub failed_at: DateTime<Utc>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum JobsMonitorError {
    #[error("event {0} not eligible for replay")]
    NotEligibleForReplay(Uuid),
    #[error("max retry attempts ({0}) exceeded")]
    MaxAttemptsExceeded(u32),
}

pub struct JobsMonitor {
    max_retries: u32,
}

impl Default for JobsMonitor {
    fn default() -> Self {
        Self { max_retries: 5 }
    }
}

impl JobsMonitor {
    pub fn new(max_retries: u32) -> Self {
        Self { max_retries }
    }

    pub fn summarize(
        &self,
        pending_count: u64,
        failed_count: u64,
        dead_letter_count: u64,
        unprocessed_events_count: u64,
        last_processed_at: Option<DateTime<Utc>>,
    ) -> OutboxJobsSummary {
        OutboxJobsSummary {
            pending_count,
            failed_count,
            dead_letter_count,
            unprocessed_events_count,
            last_processed_at,
        }
    }

    pub fn can_replay(&self, record: &FailedEventRecord) -> Result<(), JobsMonitorError> {
        if record.state != JobState::Failed && record.state != JobState::DeadLetter {
            return Err(JobsMonitorError::NotEligibleForReplay(record.event_id));
        }
        if record.attempt_count >= self.max_retries {
            return Err(JobsMonitorError::MaxAttemptsExceeded(self.max_retries));
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "platform.cockpit.jobs.tests.rs"]
mod tests;
