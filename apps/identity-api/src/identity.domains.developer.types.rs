use serde::Serialize;
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
