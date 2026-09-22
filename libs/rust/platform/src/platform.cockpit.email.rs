use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::cockpit_model::EmailOperationsSummary;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EmailDeliveryFailureItem {
    pub message_id: Uuid,
    pub business_type: String,
    pub recipient_masked: String,
    pub provider_error: String,
    pub failed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EmailSuppressionItem {
    pub email_masked: String,
    pub reason: String,
    pub suppressed_at: DateTime<Utc>,
    pub reviewed_by: Option<String>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum EmailCockpitError {
    #[error("cannot replay email {0}: status is already delivered")]
    AlreadyDelivered(Uuid),
    #[error("suppression reason required (minimum 3 chars)")]
    InvalidSuppressionReason,
}

pub struct EmailCockpitView;

impl EmailCockpitView {
    pub fn build_summary(
        queued_count: u64,
        sent_count_24h: u64,
        delivered_count_24h: u64,
        failed_count_24h: u64,
        bounce_count_24h: u64,
        active_suppressions_count: u64,
        unprocessed_events_count: u64,
    ) -> EmailOperationsSummary {
        EmailOperationsSummary {
            queued_count,
            sent_count_24h,
            delivered_count_24h,
            failed_count_24h,
            bounce_count_24h,
            active_suppressions_count,
            unprocessed_events_count,
        }
    }

    pub fn validate_suppression_request(reason: &str) -> Result<(), EmailCockpitError> {
        if reason.trim().len() < 3 {
            return Err(EmailCockpitError::InvalidSuppressionReason);
        }
        Ok(())
    }

    pub fn mask_email(email: &str) -> String {
        let parts: Vec<&str> = email.split('@').collect();
        if parts.len() != 2 {
            return "redacted".to_string();
        }
        let local = parts[0];
        let domain = parts[1];
        if local.len() <= 2 {
            format!("*@{}", domain)
        } else {
            format!("{}***{}@{}", &local[..1], &local[local.len() - 1..], domain)
        }
    }
}

#[cfg(test)]
#[path = "platform.cockpit.email.tests.rs"]
mod tests;
