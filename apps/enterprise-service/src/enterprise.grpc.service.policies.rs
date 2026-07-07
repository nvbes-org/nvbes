use tonic::{Request, Response, Status};

use crate::grpc::{
    pb::nvbes::enterprise::v1 as enterprise,
    policies, policy_evaluation,
    service_status::{non_empty, parse_uuid, validate_context},
};

pub(super) async fn get_policy_set(
    db: &sqlx::PgPool,
    request: Request<enterprise::GetPolicySetRequest>,
) -> Result<Response<enterprise::PolicySet>, Status> {
    let request = request.into_inner();
    validate_context(request.context.as_ref())?;
    let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
    Ok(Response::new(policies::policy_set(db, tenant_id).await?))
}

pub(super) async fn update_policy(
    db: &sqlx::PgPool,
    request: Request<enterprise::UpdatePolicyRequest>,
) -> Result<Response<enterprise::EnterprisePolicy>, Status> {
    let request = request.into_inner();
    validate_context(request.context.as_ref())?;
    let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
    let policy_kind = non_empty(request.policy_kind, "policy_kind")?;
    let policy = policies::update_policy(db, tenant_id, &policy_kind, &request.rules_json).await?;
    Ok(Response::new(policy))
}

pub(super) async fn evaluate_policy(
    request: Request<enterprise::EvaluatePolicyRequest>,
) -> Result<Response<enterprise::PolicyDecision>, Status> {
    let request = request.into_inner();
    validate_context(request.context.as_ref())?;
    Ok(Response::new(policy_evaluation::evaluate_policy(request)?))
}
