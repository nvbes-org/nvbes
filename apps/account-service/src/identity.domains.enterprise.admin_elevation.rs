use chrono::{DateTime, Utc};
use nvbes_core::auth::Aal;
use uuid::Uuid;

use crate::domains::auth::sessions::cache::current_session_ttl;
use crate::domains::auth::verification;
use crate::domains::enterprise::grpc::admin_elevation::{
    AuthorizeAdminElevationCommand, authorize_admin_elevation,
};
use crate::domains::enterprise::types::{
    EnterpriseAdminElevationInput, EnterpriseAdminElevationResponse, EnterpriseAdminElevationView,
    EnterpriseRole,
};
use crate::http::{error::AppError, middleware::jwt::AuthContext};

pub async fn grant_admin_elevation(
    redis: &nvbes_redis::RedisPool,
    auth: &AuthContext,
    base_role: &EnterpriseRole,
    tenant_id: Uuid,
    input: EnterpriseAdminElevationInput,
    break_glass: bool,
    break_glass_procedure: Option<(String, String)>,
) -> Result<EnterpriseAdminElevationResponse, AppError> {
    verification::require_recent_step_up(redis, auth, Some(Aal::Aal2)).await?;

    let mut session = read_session(redis, auth).await?;
    let step_up_expires_at = session
        .step_up_expires_at
        .ok_or_else(|| AppError::from(nvbes_core::auth::step_up_required_error()))?;
    let authorization = authorize_admin_elevation(AuthorizeAdminElevationCommand {
        tenant_id,
        actor_principal_id: auth.user_id,
        base_role: base_role.clone(),
        input,
        break_glass,
        break_glass_procedure,
        step_up_expires_at,
        session_expires_at: session.expires_at,
    })
    .await?;
    let expires_at = parse_grpc_time(&authorization.expires_at, "expires_at")?;
    let authorized_step_up_expires_at =
        parse_grpc_time(&authorization.step_up_expires_at, "step_up_expires_at")?;

    session.admin_elevation_role = Some("admin".to_string());
    session.admin_elevation_tenant_id = Some(tenant_id.to_string());
    session.admin_elevation_granted_at = Some(Utc::now());
    session.admin_elevation_expires_at = Some(expires_at);
    let ttl = current_session_ttl(&session);
    nvbes_redis::session::set_session(redis, &session, ttl)
        .await
        .map_err(|err| AppError::internal("redis_session_write_failed", err.to_string()))?;

    Ok(EnterpriseAdminElevationResponse {
        elevation: EnterpriseAdminElevationView {
            active: true,
            role: Some(EnterpriseRole::Admin),
            expires_at: Some(expires_at),
            step_up_expires_at: Some(authorized_step_up_expires_at),
        },
    })
}

pub async fn active_admin_elevation(
    redis: &nvbes_redis::RedisPool,
    auth: &AuthContext,
    tenant_id: Uuid,
) -> Result<Option<EnterpriseAdminElevationView>, AppError> {
    let session = read_session(redis, auth).await?;
    let now = Utc::now();
    let active = session.admin_elevation_role.as_deref() == Some("admin")
        && session.admin_elevation_tenant_id.as_deref() == Some(&tenant_id.to_string())
        && session
            .admin_elevation_expires_at
            .is_some_and(|value| value > now)
        && session.step_up_expires_at.is_some_and(|value| value > now);

    if !active {
        return Ok(None);
    }

    Ok(Some(EnterpriseAdminElevationView {
        active: true,
        role: Some(EnterpriseRole::Admin),
        expires_at: session.admin_elevation_expires_at,
        step_up_expires_at: session.step_up_expires_at,
    }))
}

pub async fn require_active_admin_elevation(
    redis: &nvbes_redis::RedisPool,
    auth: &AuthContext,
    tenant_id: Uuid,
) -> Result<EnterpriseAdminElevationView, AppError> {
    active_admin_elevation(redis, auth, tenant_id)
        .await?
        .ok_or_else(|| AppError::from(nvbes_core::auth::step_up_required_error()))
}

pub fn inactive_admin_elevation() -> EnterpriseAdminElevationView {
    EnterpriseAdminElevationView {
        active: false,
        role: None,
        expires_at: None,
        step_up_expires_at: None,
    }
}

async fn read_session(
    redis: &nvbes_redis::RedisPool,
    auth: &AuthContext,
) -> Result<nvbes_redis::session::CachedSession, AppError> {
    let session = nvbes_redis::session::get_session(redis, &auth.session_id.to_string())
        .await
        .map_err(|err| AppError::internal("redis_session_read_failed", err.to_string()))?
        .ok_or_else(|| AppError::unauthorized("session_not_found", "Session not found."))?;

    if session.principal_id != auth.user_id.to_string()
        || session.revoked_at.is_some()
        || session.expires_at <= Utc::now()
    {
        return Err(AppError::unauthorized(
            "session_expired",
            "Session expired.",
        ));
    }

    Ok(session)
}

fn parse_grpc_time(value: &str, field: &'static str) -> Result<DateTime<Utc>, AppError> {
    DateTime::parse_from_rfc3339(value.trim())
        .map(|time| time.with_timezone(&Utc))
        .map_err(|error| {
            AppError::internal(
                "enterprise_grpc_invalid_admin_elevation",
                format!("{field} from Enterprise gRPC is invalid: {error}"),
            )
        })
}

#[cfg(test)]
#[path = "identity.domains.enterprise.admin_elevation.tests.rs"]
mod tests;
