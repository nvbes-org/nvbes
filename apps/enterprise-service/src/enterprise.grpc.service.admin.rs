use sqlx::PgPool;
use tonic::{Request, Response, Status};

use crate::grpc::{
    admin_elevation, audit, invitations,
    pb::nvbes::enterprise::v1 as enterprise,
    service_status::{parse_uuid, validate_context},
    trust,
};

pub(super) async fn authorize_admin_elevation(
    db: &PgPool,
    request: Request<enterprise::AuthorizeAdminElevationRequest>,
) -> Result<Response<enterprise::AdminElevationAuthorization>, Status> {
    let request = request.into_inner();
    let context = validate_context(request.context.as_ref())?;
    let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
    let actor_id = parse_uuid(&context.actor_principal_id, "actor_principal_id")?;
    Ok(Response::new(
        admin_elevation::authorize_admin_elevation(db, tenant_id, actor_id, request).await?,
    ))
}

pub(super) async fn get_trust_center(
    db: &PgPool,
    request: Request<enterprise::GetTrustCenterRequest>,
) -> Result<Response<enterprise::TrustCenter>, Status> {
    let request = request.into_inner();
    validate_context(request.context.as_ref())?;
    let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
    Ok(Response::new(trust::trust_center(db, tenant_id).await?))
}

pub(super) async fn create_invitations(
    db: &PgPool,
    request: Request<enterprise::CreateInvitationsRequest>,
) -> Result<Response<enterprise::CreateInvitationsResponse>, Status> {
    let request = request.into_inner();
    let context = validate_context(request.context.as_ref())?;
    let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
    let actor_id = parse_uuid(&context.actor_principal_id, "actor_principal_id")?;
    Ok(Response::new(
        invitations::create_invitations(db, tenant_id, actor_id, request).await?,
    ))
}

pub(super) async fn record_developer_secret_revoked(
    db: &PgPool,
    request: Request<enterprise::RecordDeveloperSecretRevokedRequest>,
) -> Result<Response<enterprise::EnterpriseAuditRecord>, Status> {
    let request = request.into_inner();
    let context = validate_context(request.context.as_ref())?;
    let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
    let actor_id = parse_uuid(&context.actor_principal_id, "actor_principal_id")?;
    Ok(Response::new(
        audit::record_developer_secret_revoked(db, tenant_id, actor_id, request).await?,
    ))
}

pub(super) async fn list_audit_events(
    db: &PgPool,
    request: Request<enterprise::ListAuditEventsRequest>,
) -> Result<Response<enterprise::ListAuditEventsResponse>, Status> {
    let request = request.into_inner();
    validate_context(request.context.as_ref())?;
    let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
    Ok(Response::new(
        audit::list_audit_events(
            db,
            tenant_id,
            &request.scope,
            &request.organization_id,
            request.limit,
        )
        .await?,
    ))
}
