use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeveloperClientSecretRotated {
    pub tenant_id: Uuid,
    pub client_id: String,
    pub actor_principal_id: Uuid,
    pub rotated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeveloperSandboxProvisioned {
    pub tenant_id: Uuid,
    pub sandbox_id: Uuid,
    pub actor_principal_id: Uuid,
    pub provisioned_at: DateTime<Utc>,
}
