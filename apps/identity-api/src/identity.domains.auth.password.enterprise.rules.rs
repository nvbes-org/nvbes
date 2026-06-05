use chrono::{DateTime, Duration as ChronoDuration, Utc};
use uuid::Uuid;

pub fn enterprise_recovery_requires_second_approval(
    approved_by_principal_id: Option<Uuid>,
    secondary_approved_by_principal_id: Option<Uuid>,
) -> bool {
    approved_by_principal_id.is_none() && secondary_approved_by_principal_id.is_none()
}

pub fn enterprise_recovery_review_delay() -> ChronoDuration {
    ChronoDuration::hours(1)
}

pub fn enterprise_recovery_review_window_open(
    review_available_at: DateTime<Utc>,
    now: DateTime<Utc>,
) -> bool {
    review_available_at <= now
}

pub fn enterprise_recovery_approval_status_is_selectable(status: &str) -> bool {
    matches!(status, "pending" | "first_approved")
}

pub fn enterprise_recovery_first_approval_metadata(
    request_id: Uuid,
    available_at: DateTime<Utc>,
    review_available_at: DateTime<Utc>,
) -> serde_json::Value {
    serde_json::json!({
        "request_id": request_id,
        "available_at": available_at,
        "review_available_at": review_available_at,
    })
}

pub fn enterprise_recovery_review_pending_metadata(
    request_id: Uuid,
    review_available_at: DateTime<Utc>,
) -> serde_json::Value {
    serde_json::json!({
        "request_id": request_id,
        "review_available_at": review_available_at,
        "state": "pending_review",
    })
}
