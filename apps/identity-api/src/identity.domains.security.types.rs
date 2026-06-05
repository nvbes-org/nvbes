use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Deserialize, ToSchema)]
pub struct ListSecurityEventsInput {
    pub limit: Option<i64>,
    pub before_id: Option<Uuid>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SecurityEventsResponse {
    pub events: Vec<SecurityEventView>,
    pub next_cursor: Option<Uuid>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SecurityEventView {
    pub id: Uuid,
    pub event_type: String,
    pub created_at: DateTime<Utc>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SecurityExportResponse {
    pub workspace_id: Uuid,
    pub filename: String,
    pub content_type: &'static str,
    pub body: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ListRecoveryReviewsResponse {
    pub workspace_id: Uuid,
    pub requests: Vec<RecoveryReviewView>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ListRecoveryReviewsInput {
    pub limit: Option<i64>,
    pub before_created_at: Option<DateTime<Utc>>,
    pub before_id: Option<Uuid>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct WorkerQueueStatusResponse {
    pub workspace_id: Uuid,
    pub queue_name: String,
    pub snapshot_at: DateTime<Utc>,
    pub statuses: Vec<WorkerQueueStatusView>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct WorkerQueueStatusView {
    pub status: String,
    pub depth: i64,
    pub oldest_age_seconds: Option<f64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RiskEventView {
    pub id: Uuid,
    pub principal_id: Uuid,
    pub session_id: Option<Uuid>,
    pub device_id: Option<Uuid>,
    pub event_type: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub risk_score: f64,
    pub risk_factors: Value,
    pub decision: String,
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BillingWebhookEventView {
    pub provider_event_id: String,
    pub provider: String,
    pub status: String,
    pub signature_valid: bool,
    pub received_at: DateTime<Utc>,
    pub processed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RecoveryReviewView {
    pub request_id: Uuid,
    pub principal_id: Uuid,
    pub email: String,
    pub status: String,
    pub available_at: DateTime<Utc>,
    pub approved_by_principal_id: Option<Uuid>,
    pub approved_at: Option<DateTime<Utc>>,
    pub review_available_at: Option<DateTime<Utc>>,
    pub secondary_approved_by_principal_id: Option<Uuid>,
    pub secondary_approved_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::{
        ListRecoveryReviewsResponse, RecoveryReviewView, WorkerQueueStatusResponse,
        WorkerQueueStatusView,
    };
    use chrono::{TimeZone, Utc};
    use uuid::Uuid;

    #[test]
    fn recovery_reviews_response_serializes_expected_contract() {
        let request_id = Uuid::from_u128(11);
        let principal_id = Uuid::from_u128(12);
        let approved_by_principal_id = Uuid::from_u128(13);
        let secondary_approved_by_principal_id = Uuid::from_u128(14);
        let available_at = Utc
            .with_ymd_and_hms(2026, 5, 12, 8, 0, 0)
            .single()
            .expect("valid timestamp");
        let approved_at = Utc
            .with_ymd_and_hms(2026, 5, 12, 8, 15, 0)
            .single()
            .expect("valid timestamp");
        let review_available_at = Utc
            .with_ymd_and_hms(2026, 5, 12, 9, 0, 0)
            .single()
            .expect("valid timestamp");
        let secondary_approved_at = Utc
            .with_ymd_and_hms(2026, 5, 12, 9, 30, 0)
            .single()
            .expect("valid timestamp");
        let created_at = Utc
            .with_ymd_and_hms(2026, 5, 12, 8, 30, 0)
            .single()
            .expect("valid timestamp");
        let updated_at = Utc
            .with_ymd_and_hms(2026, 5, 12, 9, 45, 0)
            .single()
            .expect("valid timestamp");

        let response = ListRecoveryReviewsResponse {
            workspace_id: Uuid::from_u128(15),
            requests: vec![RecoveryReviewView {
                request_id,
                principal_id,
                email: "enterprise@example.com".to_string(),
                status: "first_approved".to_string(),
                available_at,
                approved_by_principal_id: Some(approved_by_principal_id),
                approved_at: Some(approved_at),
                review_available_at: Some(review_available_at),
                secondary_approved_by_principal_id: Some(secondary_approved_by_principal_id),
                secondary_approved_at: Some(secondary_approved_at),
                created_at,
                updated_at,
            }],
        };

        let payload = serde_json::to_value(response).expect("serializes");

        assert_eq!(payload["requests"][0]["request_id"], request_id.to_string());
        assert_eq!(
            payload["requests"][0]["principal_id"],
            principal_id.to_string()
        );
        assert_eq!(payload["requests"][0]["status"], "first_approved");
        assert_eq!(
            payload["requests"][0]["review_available_at"],
            "2026-05-12T09:00:00Z"
        );
        assert_eq!(
            payload["requests"][0]["secondary_approved_at"],
            "2026-05-12T09:30:00Z"
        );
    }

    #[test]
    fn recovery_reviews_input_deserializes_cursor_pair() {
        let input: super::ListRecoveryReviewsInput = serde_json::from_value(serde_json::json!({
            "limit": 25,
            "before_created_at": "2026-05-12T09:00:00Z",
            "before_id": Uuid::from_u128(42)
        }))
        .expect("deserializes");

        assert_eq!(input.limit, Some(25));
        assert_eq!(
            input.before_created_at,
            Some(
                Utc.with_ymd_and_hms(2026, 5, 12, 9, 0, 0)
                    .single()
                    .expect("valid timestamp")
            )
        );
        assert_eq!(input.before_id, Some(Uuid::from_u128(42)));
    }

    #[test]
    fn worker_queue_status_response_serializes_expected_contract() {
        let snapshot_at = Utc
            .with_ymd_and_hms(2026, 5, 12, 10, 0, 0)
            .single()
            .expect("valid timestamp");
        let response = WorkerQueueStatusResponse {
            workspace_id: Uuid::from_u128(99),
            queue_name: "billing.stripe.webhook.process".to_string(),
            snapshot_at,
            statuses: vec![WorkerQueueStatusView {
                status: "pending".to_string(),
                depth: 4,
                oldest_age_seconds: Some(32.5),
            }],
        };

        let payload = serde_json::to_value(response).expect("serializes");

        assert_eq!(payload["queue_name"], "billing.stripe.webhook.process");
        assert_eq!(payload["workspace_id"], Uuid::from_u128(99).to_string());
        assert_eq!(payload["snapshot_at"], "2026-05-12T10:00:00Z");
        assert_eq!(payload["statuses"][0]["status"], "pending");
        assert_eq!(payload["statuses"][0]["depth"], 4);
    }
}
