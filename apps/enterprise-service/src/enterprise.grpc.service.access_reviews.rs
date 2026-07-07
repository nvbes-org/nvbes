use sqlx::PgPool;
use tonic::{Request, Response, Status};

use crate::grpc::{
    access_reviews,
    pb::nvbes::enterprise::v1 as enterprise,
    service_status::{parse_uuid, validate_context},
};

pub(super) async fn start_access_review(
    db: &PgPool,
    request: Request<enterprise::StartAccessReviewRequest>,
) -> Result<Response<enterprise::AccessReview>, Status> {
    let request = request.into_inner();
    let context = validate_context(request.context.as_ref())?;
    let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
    let actor_id = parse_uuid(&context.actor_principal_id, "actor_principal_id")?;
    Ok(Response::new(
        access_reviews::start_access_review(db, tenant_id, actor_id, request).await?,
    ))
}

pub(super) async fn list_access_reviews(
    db: &PgPool,
    request: Request<enterprise::ListAccessReviewsRequest>,
) -> Result<Response<enterprise::ListAccessReviewsResponse>, Status> {
    let request = request.into_inner();
    validate_context(request.context.as_ref())?;
    let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
    Ok(Response::new(
        access_reviews::reads::list_access_reviews(db, tenant_id).await?,
    ))
}

pub(super) async fn get_access_review(
    db: &PgPool,
    request: Request<enterprise::GetAccessReviewRequest>,
) -> Result<Response<enterprise::AccessReviewDetail>, Status> {
    let request = request.into_inner();
    validate_context(request.context.as_ref())?;
    let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
    Ok(Response::new(
        access_reviews::reads::get_access_review(db, tenant_id, &request.review_id).await?,
    ))
}

pub(super) async fn record_access_review_decision(
    db: &PgPool,
    request: Request<enterprise::RecordAccessReviewDecisionRequest>,
) -> Result<Response<enterprise::AccessReviewDecision>, Status> {
    let request = request.into_inner();
    let context = validate_context(request.context.as_ref())?;
    let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
    let actor_id = parse_uuid(&context.actor_principal_id, "actor_principal_id")?;
    Ok(Response::new(
        access_reviews::record_access_review_decision(db, tenant_id, actor_id, request).await?,
    ))
}

pub(super) async fn close_access_review(
    db: &PgPool,
    request: Request<enterprise::CloseAccessReviewRequest>,
) -> Result<Response<enterprise::AccessReview>, Status> {
    let request = request.into_inner();
    validate_context(request.context.as_ref())?;
    let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
    Ok(Response::new(
        access_reviews::close_access_review(db, tenant_id, request).await?,
    ))
}

pub(super) async fn list_access_review_schedules(
    db: &PgPool,
    request: Request<enterprise::ListAccessReviewSchedulesRequest>,
) -> Result<Response<enterprise::ListAccessReviewSchedulesResponse>, Status> {
    let request = request.into_inner();
    validate_context(request.context.as_ref())?;
    let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
    Ok(Response::new(
        access_reviews::schedules::list_access_review_schedules(db, tenant_id).await?,
    ))
}

pub(super) async fn create_access_review_schedule(
    db: &PgPool,
    request: Request<enterprise::CreateAccessReviewScheduleRequest>,
) -> Result<Response<enterprise::AccessReviewSchedule>, Status> {
    let request = request.into_inner();
    let context = validate_context(request.context.as_ref())?;
    let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
    let actor_id = parse_uuid(&context.actor_principal_id, "actor_principal_id")?;
    Ok(Response::new(
        access_reviews::schedules::create_access_review_schedule(db, tenant_id, actor_id, request)
            .await?,
    ))
}

pub(super) async fn set_access_review_schedule_enabled(
    db: &PgPool,
    request: Request<enterprise::SetAccessReviewScheduleEnabledRequest>,
) -> Result<Response<enterprise::AccessReviewSchedule>, Status> {
    let request = request.into_inner();
    validate_context(request.context.as_ref())?;
    let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
    let schedule_id = parse_uuid(&request.schedule_id, "schedule_id")?;
    Ok(Response::new(
        access_reviews::schedules::set_access_review_schedule_enabled(
            db,
            tenant_id,
            schedule_id,
            request.enabled,
        )
        .await?,
    ))
}

pub(super) async fn run_access_review_schedule(
    db: &PgPool,
    request: Request<enterprise::RunAccessReviewScheduleRequest>,
) -> Result<Response<enterprise::AccessReview>, Status> {
    let request = request.into_inner();
    let context = validate_context(request.context.as_ref())?;
    let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
    let schedule_id = parse_uuid(&request.schedule_id, "schedule_id")?;
    let actor_id = parse_uuid(&context.actor_principal_id, "actor_principal_id")?;
    Ok(Response::new(
        access_reviews::schedules::run_access_review_schedule(db, tenant_id, schedule_id, actor_id)
            .await?,
    ))
}

pub(super) async fn materialize_due_access_review_schedules(
    db: &PgPool,
    request: Request<enterprise::MaterializeDueAccessReviewSchedulesRequest>,
) -> Result<Response<enterprise::AccessReviewScheduleRun>, Status> {
    let request = request.into_inner();
    validate_context(request.context.as_ref())?;
    Ok(Response::new(
        access_reviews::schedules::materialize_due_schedules(db, request).await?,
    ))
}

pub(super) async fn claim_access_review_reminder_candidates(
    db: &PgPool,
    request: Request<enterprise::ClaimAccessReviewReminderCandidatesRequest>,
) -> Result<Response<enterprise::AccessReviewReminderClaim>, Status> {
    let request = request.into_inner();
    validate_context(request.context.as_ref())?;
    Ok(Response::new(
        access_reviews::reminders::claim_reminder_candidates(db, request).await?,
    ))
}
