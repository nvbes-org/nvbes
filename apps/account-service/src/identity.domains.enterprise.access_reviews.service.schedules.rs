use uuid::Uuid;

use crate::database::Database;
use crate::domains::enterprise::access_reviews::{types::*, validation};
use crate::domains::enterprise::grpc;
use crate::http::error::AppError;
use crate::http::middleware::jwt::AuthContext;

pub async fn list_schedules(
    db: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
) -> Result<AccessReviewSchedulesResponse, AppError> {
    super::ensure_tenant_admin(db, auth, tenant_id).await?;
    let response =
        grpc::access_reviews::schedules::list_access_review_schedules(tenant_id, auth.user_id)
            .await?;
    Ok(AccessReviewSchedulesResponse {
        schedules: response
            .schedules
            .into_iter()
            .map(schedule_from_grpc)
            .collect::<Result<_, _>>()?,
    })
}

pub async fn create_schedule(
    db: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
    input: CreateAccessReviewScheduleInput,
) -> Result<AccessReviewSchedule, AppError> {
    super::ensure_tenant_admin(db, auth, tenant_id).await?;
    validation::validate_schedule_input(&input)?;

    let schedule = grpc::access_reviews::schedules::create_access_review_schedule(
        tenant_id,
        auth.user_id,
        &input,
    )
    .await?;
    schedule_from_grpc(schedule)
}

pub async fn materialize_due_schedules(
    db: &Database,
) -> Result<crate::grpc_pb::nvbes::enterprise::v1::AccessReviewScheduleRun, AppError> {
    let _ = db;
    grpc::access_reviews::schedules::materialize_due_access_review_schedules(25).await
}

pub async fn disable_schedule(
    db: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
    schedule_id: Uuid,
) -> Result<AccessReviewSchedule, AppError> {
    super::ensure_tenant_admin(db, auth, tenant_id).await?;
    let schedule = grpc::access_reviews::schedules::set_access_review_schedule_enabled(
        tenant_id,
        auth.user_id,
        schedule_id,
        false,
    )
    .await?;
    schedule_from_grpc(schedule)
}

pub async fn enable_schedule(
    db: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
    schedule_id: Uuid,
) -> Result<AccessReviewSchedule, AppError> {
    super::ensure_tenant_admin(db, auth, tenant_id).await?;
    let schedule = grpc::access_reviews::schedules::set_access_review_schedule_enabled(
        tenant_id,
        auth.user_id,
        schedule_id,
        true,
    )
    .await?;
    schedule_from_grpc(schedule)
}

pub async fn run_schedule_now(
    db: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
    schedule_id: Uuid,
) -> Result<AccessReviewCampaignDetail, AppError> {
    super::ensure_tenant_admin(db, auth, tenant_id).await?;
    let campaign_id = grpc::access_reviews::schedules::run_access_review_schedule(
        tenant_id,
        auth.user_id,
        schedule_id,
    )
    .await?;
    super::campaign_detail(tenant_id, auth.user_id, campaign_id).await
}

fn schedule_from_grpc(
    schedule: crate::grpc_pb::nvbes::enterprise::v1::AccessReviewSchedule,
) -> Result<AccessReviewSchedule, AppError> {
    Ok(AccessReviewSchedule {
        id: parse_uuid(&schedule.schedule_id, "schedule_id")?,
        name: schedule.name,
        description: optional_text(schedule.description),
        recurrence_days: schedule.recurrence_days,
        due_after_days: schedule.due_after_days,
        next_run_at: parse_time(&schedule.next_run_at, "next_run_at")?,
        last_campaign_id: optional_uuid(&schedule.last_review_id, "last_review_id")?,
        created_by: parse_uuid(&schedule.created_by, "created_by")?,
        created_at: parse_time(&schedule.created_at, "created_at")?,
        disabled_at: optional_time(&schedule.disabled_at, "disabled_at")?,
        scope: AccessReviewCampaignScopeInput {
            include_members: schedule.include_members,
            include_roles: schedule.include_roles,
            include_service_accounts: schedule.include_service_accounts,
            include_oauth_clients: schedule.include_oauth_clients,
        },
    })
}

fn parse_uuid(value: &str, field: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(value).map_err(|error| {
        AppError::internal(
            "enterprise_grpc_invalid_access_review_schedule",
            format!("Enterprise gRPC returned invalid {field}: {error}"),
        )
    })
}

fn optional_uuid(value: &str, field: &str) -> Result<Option<Uuid>, AppError> {
    if value.trim().is_empty() {
        Ok(None)
    } else {
        parse_uuid(value, field).map(Some)
    }
}

fn parse_time(value: &str, field: &str) -> Result<chrono::DateTime<chrono::Utc>, AppError> {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&chrono::Utc))
        .map_err(|error| {
            AppError::internal(
                "enterprise_grpc_invalid_access_review_schedule",
                format!("Enterprise gRPC returned invalid {field}: {error}"),
            )
        })
}

fn optional_time(
    value: &str,
    field: &str,
) -> Result<Option<chrono::DateTime<chrono::Utc>>, AppError> {
    if value.trim().is_empty() {
        Ok(None)
    } else {
        parse_time(value, field).map(Some)
    }
}

fn optional_text(value: String) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}
