use sqlx::PgPool;
use tonic::{Request, Response, Status};

use crate::grpc::{
    break_glass,
    pb::nvbes::enterprise::v1 as enterprise,
    privileged_authentication,
    service_status::{non_empty, parse_uuid, validate_context},
};

pub(super) async fn activate_break_glass(
    db: &PgPool,
    request: Request<enterprise::ActivateBreakGlassRequest>,
) -> Result<Response<enterprise::BreakGlassGrant>, Status> {
    let request = request.into_inner();
    let context = validate_context(request.context.as_ref())?;
    privileged_authentication::require(request.authentication.as_ref())?;
    let authentication = request
        .authentication
        .as_ref()
        .expect("privileged authentication was validated");
    let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
    let principal_id = parse_uuid(&request.principal_id, "principal_id")?;
    let actor_id = parse_uuid(&context.actor_principal_id, "actor_principal_id")?;
    let reason = non_empty(request.reason, "reason")?;
    let procedure_reference = non_empty(request.role, "role")?;
    break_glass::upsert_break_glass(
        db,
        tenant_id,
        principal_id,
        actor_id,
        &procedure_reference,
        &reason,
        authentication,
    )
    .await?;
    Ok(Response::new(enterprise::BreakGlassGrant {
        grant_id: principal_id.to_string(),
        tenant_id: tenant_id.to_string(),
        principal_id: principal_id.to_string(),
        role: procedure_reference,
        status: "active".to_string(),
        expires_at: request.expires_at,
    }))
}

pub(super) async fn revoke_break_glass(
    db: &PgPool,
    request: Request<enterprise::RevokeBreakGlassRequest>,
) -> Result<Response<enterprise::BreakGlassGrant>, Status> {
    let request = request.into_inner();
    let context = validate_context(request.context.as_ref())?;
    let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
    let principal_id = parse_uuid(&request.grant_id, "grant_id")?;
    let actor_id = parse_uuid(&context.actor_principal_id, "actor_principal_id")?;
    let reason = non_empty(request.reason, "reason")?;
    let changed =
        break_glass::revoke_break_glass(db, tenant_id, principal_id, actor_id, &reason).await?;
    if changed == 0 {
        return Err(Status::not_found(
            "enterprise break-glass grant was not found",
        ));
    }
    Ok(Response::new(enterprise::BreakGlassGrant {
        grant_id: principal_id.to_string(),
        tenant_id: tenant_id.to_string(),
        principal_id: principal_id.to_string(),
        role: String::new(),
        status: "revoked".to_string(),
        expires_at: String::new(),
    }))
}

pub(super) async fn list_break_glass_accounts(
    db: &PgPool,
    request: Request<enterprise::ListBreakGlassAccountsRequest>,
) -> Result<Response<enterprise::ListBreakGlassAccountsResponse>, Status> {
    let request = request.into_inner();
    validate_context(request.context.as_ref())?;
    let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
    let principal_ids = request
        .principal_ids
        .iter()
        .map(|value| parse_uuid(value, "principal_id"))
        .collect::<Result<Vec<_>, _>>()?;
    let accounts = break_glass::list_break_glass_accounts(db, tenant_id, &principal_ids).await?;
    Ok(Response::new(enterprise::ListBreakGlassAccountsResponse {
        accounts,
    }))
}
