use crate::app::AppState;
use crate::domains::auth::sessions;
use crate::domains::auth::types::AuthContext;
use crate::http::error::AppError;
use axum::{Router, http::HeaderMap};
use uuid::Uuid;

#[path = "identity.domains.federation.routes.domains.rs"]
pub mod domains;
#[path = "identity.domains.federation.routes.identities.rs"]
pub mod identities;
#[path = "identity.domains.federation.routes.idp.rs"]
pub mod idp;
#[path = "identity.domains.federation.routes.protocol.rs"]
pub mod protocol;
#[path = "identity.domains.federation.routes.scim.rs"]
pub mod scim;

pub fn router(_state: &AppState) -> Router<AppState> {
    Router::new()
        .merge(domains::router())
        .merge(idp::router())
        .merge(scim::router())
        .merge(protocol::router())
        .merge(identities::router())
}

pub async fn authenticate_tenant(
    state: &AppState,
    headers: &HeaderMap,
    tenant_id: Uuid,
) -> Result<AuthContext, AppError> {
    let auth = sessions::authenticate_bearer(&state.db, &state.redis, &state.jwt, headers).await?;
    if auth.email_verified_at.is_none() {
        return Err(AppError::forbidden(
            crate::domains::federation::contract::EMAIL_NOT_VERIFIED,
            "Verify your email address before using federation management.",
        ));
    }

    let current_tenant = auth.tenant_id.ok_or_else(|| {
        AppError::forbidden(
            crate::domains::federation::contract::TENANT_CONTEXT_REQUIRED,
            "A tenant context is required before using federation management.",
        )
    })?;

    if current_tenant != tenant_id {
        return Err(AppError::forbidden(
            crate::domains::federation::contract::TENANT_MISMATCH,
            "This tenant does not match the active tenant context.",
        ));
    }

    crate::domains::authz::ensure_tenant_management_access(&state.db, &auth, tenant_id).await?;

    Ok(auth)
}
