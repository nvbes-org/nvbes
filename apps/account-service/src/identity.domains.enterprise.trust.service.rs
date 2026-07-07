use uuid::Uuid;

use super::types::EnterpriseTrustCenterResponse;
use crate::database::Database;
use crate::domains::enterprise::grpc;
use crate::http::error::AppError;
use crate::http::middleware::jwt::AuthContext;

use crate::domains::authz::{AdminScope, resolve_admin_scope};

pub async fn get_trust_center(
    pool: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
) -> Result<EnterpriseTrustCenterResponse, AppError> {
    let scope = resolve_admin_scope(pool, auth, tenant_id, auth.organization_id).await?;
    if !matches!(scope, AdminScope::Tenant) {
        return Err(AppError::forbidden(
            "tenant_scope_required",
            "This action requires tenant-wide administrative privileges.",
        ));
    }
    grpc::trust::get_trust_center(tenant_id, auth.user_id).await
}
