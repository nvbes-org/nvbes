use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnterpriseAccessReviewCampaignCreated {
    pub tenant_id: Uuid,
    pub campaign_id: Uuid,
    pub actor_principal_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnterpriseAccessChanged {
    pub tenant_id: Uuid,
    pub principal_id: Uuid,
    pub actor_principal_id: Uuid,
    pub changed_at: DateTime<Utc>,
}
