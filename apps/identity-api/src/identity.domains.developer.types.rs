use serde::Serialize;
use utoipa::ToSchema;

use super::rbac::{DeveloperPermission, DeveloperRole};

#[derive(Debug, Serialize, ToSchema)]
pub struct DeveloperMeResponse {
    pub tenant_id: String,
    pub roles: Vec<DeveloperRole>,
    pub permissions: Vec<DeveloperPermission>,
}
