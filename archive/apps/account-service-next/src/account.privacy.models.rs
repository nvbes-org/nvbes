use serde::Serialize;

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct GpcStatus {
    pub gpc_enabled: bool,
    pub gpc_opt_out_active: bool,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct SuccessResponse {
    pub success: bool,
}

impl SuccessResponse {
    pub fn ok() -> Self {
        Self { success: true }
    }
}
