use sqlx::PgPool;
use tonic::{Request, Response, Status};

use crate::grpc::{
    pb::nvbes::enterprise::v1 as enterprise,
    service_status::{parse_uuid, validate_context},
    user_access,
};

pub(super) async fn update_user_access(
    db: &PgPool,
    request: Request<enterprise::UpdateUserAccessRequest>,
) -> Result<Response<enterprise::UserAccessChange>, Status> {
    let request = request.into_inner();
    let context = validate_context(request.context.as_ref())?;
    let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
    let actor_id = parse_uuid(&context.actor_principal_id, "actor_principal_id")?;
    Ok(Response::new(
        user_access::update_user_access(db, tenant_id, actor_id, request).await?,
    ))
}

pub(super) async fn suspend_user_access(
    db: &PgPool,
    request: Request<enterprise::SuspendUserAccessRequest>,
) -> Result<Response<enterprise::UserAccessChange>, Status> {
    let request = request.into_inner();
    let context = validate_context(request.context.as_ref())?;
    let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
    let actor_id = parse_uuid(&context.actor_principal_id, "actor_principal_id")?;
    Ok(Response::new(
        user_access::suspend_user_access(db, tenant_id, actor_id, request).await?,
    ))
}

pub(super) async fn reactivate_user_access(
    db: &PgPool,
    request: Request<enterprise::ReactivateUserAccessRequest>,
) -> Result<Response<enterprise::UserAccessChange>, Status> {
    let request = request.into_inner();
    let context = validate_context(request.context.as_ref())?;
    let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
    let actor_id = parse_uuid(&context.actor_principal_id, "actor_principal_id")?;
    Ok(Response::new(
        user_access::reactivate_user_access(db, tenant_id, actor_id, request).await?,
    ))
}
