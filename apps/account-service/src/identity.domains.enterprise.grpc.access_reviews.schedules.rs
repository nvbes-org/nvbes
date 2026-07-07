use uuid::Uuid;

use crate::{
    domains::enterprise::access_reviews::types::{
        AccessReviewCampaignScopeInput, CreateAccessReviewScheduleInput,
    },
    grpc_pb::nvbes::enterprise::v1::{
        self as enterprise, CreateAccessReviewScheduleRequest, ListAccessReviewSchedulesRequest,
        MaterializeDueAccessReviewSchedulesRequest, RunAccessReviewScheduleRequest,
        SetAccessReviewScheduleEnabledRequest,
    },
    http::error::AppError,
};

pub async fn list_access_review_schedules(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
) -> Result<enterprise::ListAccessReviewSchedulesResponse, AppError> {
    let mut client = super::super::enterprise_client().await?;
    let response = client
        .list_access_review_schedules(ListAccessReviewSchedulesRequest {
            context: Some(super::super::request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
        })
        .await
        .map_err(super::super::grpc_error)?;
    Ok(response.into_inner())
}

pub async fn create_access_review_schedule(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    input: &CreateAccessReviewScheduleInput,
) -> Result<enterprise::AccessReviewSchedule, AppError> {
    let mut client = super::super::enterprise_client().await?;
    let response = client
        .create_access_review_schedule(CreateAccessReviewScheduleRequest {
            context: Some(super::super::request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
            name: input.name.clone(),
            description: input.description.clone().unwrap_or_default(),
            scope: access_review_scope_label(&input.scope).to_string(),
            recurrence_days: input.recurrence_days,
            due_after_days: input.due_after_days,
            include_members: input.scope.include_members,
            include_roles: input.scope.include_roles,
            include_service_accounts: input.scope.include_service_accounts,
            include_oauth_clients: input.scope.include_oauth_clients,
        })
        .await
        .map_err(super::super::grpc_error)?;
    Ok(response.into_inner())
}

pub async fn set_access_review_schedule_enabled(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    schedule_id: Uuid,
    enabled: bool,
) -> Result<enterprise::AccessReviewSchedule, AppError> {
    let mut client = super::super::enterprise_client().await?;
    let response = client
        .set_access_review_schedule_enabled(SetAccessReviewScheduleEnabledRequest {
            context: Some(super::super::request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
            schedule_id: schedule_id.to_string(),
            enabled,
        })
        .await
        .map_err(super::super::grpc_error)?;
    Ok(response.into_inner())
}

pub async fn run_access_review_schedule(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    schedule_id: Uuid,
) -> Result<Uuid, AppError> {
    let mut client = super::super::enterprise_client().await?;
    let response = client
        .run_access_review_schedule(RunAccessReviewScheduleRequest {
            context: Some(super::super::request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
            schedule_id: schedule_id.to_string(),
        })
        .await
        .map_err(super::super::grpc_error)?
        .into_inner();
    Uuid::parse_str(&response.review_id).map_err(|error| {
        AppError::internal(
            "enterprise_grpc_invalid_access_review_id",
            format!("Enterprise gRPC returned an invalid access review ID: {error}"),
        )
    })
}

pub async fn materialize_due_access_review_schedules(
    limit: i32,
) -> Result<enterprise::AccessReviewScheduleRun, AppError> {
    let mut client = super::super::enterprise_client().await?;
    let response = client
        .materialize_due_access_review_schedules(MaterializeDueAccessReviewSchedulesRequest {
            context: Some(super::super::request_context(Uuid::nil(), Uuid::nil())),
            limit,
        })
        .await
        .map_err(super::super::grpc_error)?;
    Ok(response.into_inner())
}

fn access_review_scope_label(scope: &AccessReviewCampaignScopeInput) -> &'static str {
    match (
        scope.include_members,
        scope.include_roles,
        scope.include_service_accounts,
        scope.include_oauth_clients,
    ) {
        (true, true, true, true) => "all",
        (true, false, false, false) => "members",
        (false, true, false, false) => "roles",
        (false, false, true, false) => "service_accounts",
        (false, false, false, true) => "oauth_clients",
        _ => "custom",
    }
}
