use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::rbac::{DeveloperPermission, DeveloperRole};

#[derive(Debug, Serialize, ToSchema)]
pub struct DeveloperMeResponse {
    pub tenant_id: Uuid,
    pub roles: Vec<DeveloperRole>,
    pub permissions: Vec<DeveloperPermission>,
}

impl DeveloperMeResponse {
    pub fn new(
        tenant_id: Uuid,
        roles: Vec<DeveloperRole>,
        permissions: Vec<DeveloperPermission>,
    ) -> Self {
        Self {
            tenant_id,
            roles,
            permissions,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DeveloperAppView {
    pub id: Uuid,
    pub client_id: String,
    pub name: String,
    pub redirect_uris: Vec<String>,
    pub client_type: String,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DeveloperAppsResponse {
    pub apps: Vec<DeveloperAppView>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateDeveloperAppRequest {
    pub name: String,
    pub redirect_uris: Vec<String>,
    pub allowed_scopes: Vec<String>,
    #[serde(default)]
    pub allowed_audiences: Vec<String>,
    #[serde(default)]
    pub allowed_resources: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreateDeveloperAppResponse {
    pub app: DeveloperAppView,
    pub client_secret: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateDeveloperRedirectsRequest {
    pub redirect_uris: Vec<String>,
}
