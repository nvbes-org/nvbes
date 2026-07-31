use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ClaimedClosure {
    pub event_id: Uuid,
    pub saga_id: Uuid,
    pub principal_id: Uuid,
    pub requested_at: DateTime<Utc>,
    pub avatar_object_key: Option<String>,
    pub attempt: i32,
}

#[derive(Debug, Serialize)]
pub struct IdentityClosureRequest {
    pub event_id: Uuid,
    pub saga_id: Uuid,
    pub principal_id: Uuid,
    pub requested_at: DateTime<Utc>,
}

impl From<&ClaimedClosure> for IdentityClosureRequest {
    fn from(event: &ClaimedClosure) -> Self {
        Self {
            event_id: event.event_id,
            saga_id: event.saga_id,
            principal_id: event.principal_id,
            requested_at: event.requested_at,
        }
    }
}
