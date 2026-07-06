use sqlx::PgPool;

pub use super::scheduler_types::{AccessReviewReminderRun, AccessReviewScheduleRun};
use crate::AccountResult;

const DUE_SCHEDULE_LIMIT: i64 = 25;
const REMINDER_LIMIT: i64 = 100;

pub async fn materialize_due_schedules(db: &PgPool) -> AccountResult<AccessReviewScheduleRun> {
    super::scheduler_campaigns::materialize_due_schedules(db, DUE_SCHEDULE_LIMIT).await
}

pub async fn run_schedule_now(
    db: &PgPool,
    tenant_id: uuid::Uuid,
    actor_id: uuid::Uuid,
    schedule_id: uuid::Uuid,
) -> AccountResult<uuid::Uuid> {
    super::scheduler_campaigns::run_schedule_now(db, tenant_id, actor_id, schedule_id).await
}

pub async fn enqueue_due_campaign_reminders(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    config: &nvbes_core::config::AppConfig,
) -> AccountResult<AccessReviewReminderRun> {
    super::scheduler_reminders::enqueue_due_campaign_reminders(db, redis, config, REMINDER_LIMIT)
        .await
}
