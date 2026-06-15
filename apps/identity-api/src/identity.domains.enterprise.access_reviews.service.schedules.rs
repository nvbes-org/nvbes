use uuid::Uuid;

use crate::database::Database;
use crate::domains::enterprise::access_reviews::{
    reminders, schedule_mutations, schedule_state, schedules, types::*, validation,
};
use crate::domains::enterprise::service::require_actor_access_for_enterprise;
use crate::http::error::AppError;
use crate::http::middleware::jwt::AuthContext;

pub async fn list_schedules(
    db: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
) -> Result<AccessReviewSchedulesResponse, AppError> {
    crate::domains::authz::ensure_tenant_management_access(db, auth, tenant_id).await?;
    require_actor_access_for_enterprise(db, auth, tenant_id).await?;
    Ok(AccessReviewSchedulesResponse {
        schedules: schedules::list_schedules(db, tenant_id)
            .await?
            .into_iter()
            .map(schedules::AccessReviewScheduleRow::into_view)
            .collect(),
    })
}

pub async fn create_schedule(
    db: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
    input: CreateAccessReviewScheduleInput,
) -> Result<AccessReviewSchedule, AppError> {
    crate::domains::authz::ensure_tenant_management_access(db, auth, tenant_id).await?;
    require_actor_access_for_enterprise(db, auth, tenant_id).await?;
    validation::validate_schedule_input(&input)?;

    let mut tx = db.begin().await?;
    let schedule_id = schedules::insert_schedule(&mut tx, tenant_id, auth.user_id, &input).await?;
    crate::domains::enterprise::db::insert_audit(
        &mut tx,
        tenant_id,
        auth.user_id,
        "enterprise.access_review_schedule.created",
        "access_review_schedule",
        Some(schedule_id),
        serde_json::json!({
            "name": input.name.trim(),
            "recurrence_days": input.recurrence_days,
            "due_after_days": input.due_after_days,
            "scope": input.scope
        }),
    )
    .await?;
    tx.commit().await?;

    schedules::get_schedule(db, tenant_id, schedule_id)
        .await?
        .map(schedules::AccessReviewScheduleRow::into_view)
        .ok_or_else(|| {
            AppError::not_found(
                "access_review_schedule_not_found",
                "Access review schedule not found.",
            )
        })
}

pub async fn materialize_due_schedules(
    db: &Database,
) -> Result<schedules::AccessReviewScheduleRun, AppError> {
    schedule_mutations::materialize_due_schedules(db, 25).await
}

pub async fn enqueue_due_campaign_reminders(
    db: &Database,
    redis: &nvbes_redis::RedisPool,
    config: &crate::app::AppConfig,
) -> Result<reminders::AccessReviewReminderRun, AppError> {
    reminders::enqueue_due_campaign_reminders(db, redis, config, 100).await
}

pub async fn disable_schedule(
    db: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
    schedule_id: Uuid,
) -> Result<AccessReviewSchedule, AppError> {
    crate::domains::authz::ensure_tenant_management_access(db, auth, tenant_id).await?;
    require_actor_access_for_enterprise(db, auth, tenant_id).await?;
    if !schedule_state::disable_schedule(db, tenant_id, schedule_id).await? {
        return Err(AppError::not_found(
            "access_review_schedule_not_found",
            "Access review schedule not found.",
        ));
    }
    schedules::get_schedule(db, tenant_id, schedule_id)
        .await?
        .map(schedules::AccessReviewScheduleRow::into_view)
        .ok_or_else(|| {
            AppError::not_found(
                "access_review_schedule_not_found",
                "Access review schedule not found.",
            )
        })
}

pub async fn enable_schedule(
    db: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
    schedule_id: Uuid,
) -> Result<AccessReviewSchedule, AppError> {
    crate::domains::authz::ensure_tenant_management_access(db, auth, tenant_id).await?;
    require_actor_access_for_enterprise(db, auth, tenant_id).await?;
    if !schedule_state::enable_schedule(db, tenant_id, schedule_id).await? {
        return Err(AppError::not_found(
            "access_review_schedule_not_found",
            "Access review schedule not found.",
        ));
    }
    schedules::get_schedule(db, tenant_id, schedule_id)
        .await?
        .map(schedules::AccessReviewScheduleRow::into_view)
        .ok_or_else(|| {
            AppError::not_found(
                "access_review_schedule_not_found",
                "Access review schedule not found.",
            )
        })
}

pub async fn run_schedule_now(
    db: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
    schedule_id: Uuid,
) -> Result<AccessReviewCampaignDetail, AppError> {
    crate::domains::authz::ensure_tenant_management_access(db, auth, tenant_id).await?;
    require_actor_access_for_enterprise(db, auth, tenant_id).await?;
    let campaign_id =
        schedule_mutations::run_schedule_now(db, tenant_id, auth.user_id, schedule_id).await?;
    super::campaign_detail(db, tenant_id, campaign_id).await
}
