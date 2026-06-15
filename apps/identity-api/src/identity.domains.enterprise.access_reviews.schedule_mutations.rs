use chrono::{DateTime, Duration, Utc};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::{db, schedules::AccessReviewScheduleRun, types::*};
use crate::http::error::AppError;

pub async fn materialize_due_schedules(
    db: &PgPool,
    limit: i64,
) -> Result<AccessReviewScheduleRun, AppError> {
    let mut tx = db.begin().await?;
    let schedules = claim_due_schedules(&mut tx, limit).await?;
    let mut run = AccessReviewScheduleRun::default();
    let now = Utc::now();

    for schedule in schedules {
        match execute_schedule(&mut tx, &schedule, schedule.created_by, now).await? {
            ScheduleExecution::Created {
                campaign_id,
                due_at,
                item_count,
            } => {
                update_after_run(
                    &mut tx,
                    schedule.id,
                    Some(campaign_id),
                    next_run_at(now, schedule.recurrence_days),
                )
                .await?;
                audit_created_campaign(
                    &mut tx,
                    &schedule,
                    schedule.created_by,
                    campaign_id,
                    item_count,
                    due_at,
                )
                .await?;
                run.campaigns_created += 1;
            }
            ScheduleExecution::Empty => {
                update_after_run(
                    &mut tx,
                    schedule.id,
                    None,
                    next_run_at(now, schedule.recurrence_days),
                )
                .await?;
                run.empty_schedules += 1;
            }
        }
    }

    tx.commit().await?;
    Ok(run)
}

pub async fn run_schedule_now(
    db: &PgPool,
    tenant_id: Uuid,
    actor_id: Uuid,
    schedule_id: Uuid,
) -> Result<Uuid, AppError> {
    let mut tx = db.begin().await?;
    let schedule = get_schedule_for_update(&mut tx, tenant_id, schedule_id)
        .await?
        .ok_or_else(|| {
            AppError::not_found(
                "access_review_schedule_not_found",
                "Access review schedule not found.",
            )
        })?;

    if schedule.disabled_at.is_some() {
        return Err(AppError::conflict(
            "access_review_schedule_disabled",
            "Disabled access review schedules cannot be run.",
        ));
    }

    let now = Utc::now();
    match execute_schedule(&mut tx, &schedule, actor_id, now).await? {
        ScheduleExecution::Created {
            campaign_id,
            due_at,
            item_count,
        } => {
            update_after_run(
                &mut tx,
                schedule.id,
                Some(campaign_id),
                next_run_at(now, schedule.recurrence_days),
            )
            .await?;
            audit_created_campaign(
                &mut tx,
                &schedule,
                actor_id,
                campaign_id,
                item_count,
                due_at,
            )
            .await?;
            tx.commit().await?;
            Ok(campaign_id)
        }
        ScheduleExecution::Empty => Err(AppError::bad_request(
            "empty_access_review_scope",
            "The selected access review scope does not contain any reviewable access.",
        )),
    }
}

async fn claim_due_schedules(
    tx: &mut Transaction<'_, Postgres>,
    limit: i64,
) -> Result<Vec<ScheduleExecutionRow>, AppError> {
    Ok(sqlx::query_as::<_, ScheduleExecutionRow>(
        r#"
        SELECT id, tenant_id, name, description, include_members, include_roles,
          include_service_accounts, include_oauth_clients, recurrence_days,
          due_after_days, created_by, disabled_at
        FROM access_review_schedules
        WHERE disabled_at IS NULL AND next_run_at <= NOW()
        ORDER BY next_run_at ASC, created_at ASC
        LIMIT $1
        FOR UPDATE SKIP LOCKED
        "#,
    )
    .bind(limit)
    .fetch_all(&mut **tx)
    .await?)
}

async fn get_schedule_for_update(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    schedule_id: Uuid,
) -> Result<Option<ScheduleExecutionRow>, AppError> {
    Ok(sqlx::query_as::<_, ScheduleExecutionRow>(
        r#"
        SELECT id, tenant_id, name, description, include_members, include_roles,
          include_service_accounts, include_oauth_clients, recurrence_days,
          due_after_days, created_by, disabled_at
        FROM access_review_schedules
        WHERE tenant_id = $1 AND id = $2
        FOR UPDATE
        "#,
    )
    .bind(tenant_id)
    .bind(schedule_id)
    .fetch_optional(&mut **tx)
    .await?)
}

async fn execute_schedule(
    tx: &mut Transaction<'_, Postgres>,
    schedule: &ScheduleExecutionRow,
    created_by: Uuid,
    now: DateTime<Utc>,
) -> Result<ScheduleExecution, AppError> {
    let due_at = now + Duration::days(schedule.due_after_days.into());
    let campaign_id = db::insert_campaign(
        tx,
        schedule.tenant_id,
        created_by,
        &schedule.name,
        schedule.description.as_deref(),
        due_at,
    )
    .await?;
    let item_count =
        db::insert_snapshot_items(tx, campaign_id, schedule.tenant_id, &schedule.scope()).await?;
    if item_count == 0 {
        delete_empty_campaign(tx, campaign_id).await?;
        return Ok(ScheduleExecution::Empty);
    }
    Ok(ScheduleExecution::Created {
        campaign_id,
        due_at,
        item_count,
    })
}

async fn update_after_run(
    tx: &mut Transaction<'_, Postgres>,
    schedule_id: Uuid,
    campaign_id: Option<Uuid>,
    next_run_at: DateTime<Utc>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE access_review_schedules
        SET last_campaign_id = COALESCE($2, last_campaign_id),
          next_run_at = $3
        WHERE id = $1
        "#,
    )
    .bind(schedule_id)
    .bind(campaign_id)
    .bind(next_run_at)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn delete_empty_campaign(
    tx: &mut Transaction<'_, Postgres>,
    campaign_id: Uuid,
) -> Result<(), AppError> {
    sqlx::query("DELETE FROM access_review_campaigns WHERE id = $1")
        .bind(campaign_id)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

async fn audit_created_campaign(
    tx: &mut Transaction<'_, Postgres>,
    schedule: &ScheduleExecutionRow,
    actor_id: Uuid,
    campaign_id: Uuid,
    item_count: u64,
    due_at: DateTime<Utc>,
) -> Result<(), AppError> {
    crate::domains::enterprise::db::insert_audit(
        tx,
        schedule.tenant_id,
        actor_id,
        "enterprise.access_review_campaign.created",
        "access_review_campaign",
        Some(campaign_id),
        serde_json::json!({
            "name": &schedule.name,
            "due_at": due_at,
            "item_count": item_count,
            "schedule_id": schedule.id
        }),
    )
    .await
}

fn next_run_at(now: DateTime<Utc>, recurrence_days: i32) -> DateTime<Utc> {
    now + Duration::days(recurrence_days.into())
}

enum ScheduleExecution {
    Created {
        campaign_id: Uuid,
        due_at: DateTime<Utc>,
        item_count: u64,
    },
    Empty,
}

#[derive(Debug, FromRow)]
struct ScheduleExecutionRow {
    id: Uuid,
    tenant_id: Uuid,
    name: String,
    description: Option<String>,
    include_members: bool,
    include_roles: bool,
    include_service_accounts: bool,
    include_oauth_clients: bool,
    recurrence_days: i32,
    due_after_days: i32,
    created_by: Uuid,
    disabled_at: Option<DateTime<Utc>>,
}

impl ScheduleExecutionRow {
    fn scope(&self) -> AccessReviewCampaignScopeInput {
        AccessReviewCampaignScopeInput {
            include_members: self.include_members,
            include_roles: self.include_roles,
            include_service_accounts: self.include_service_accounts,
            include_oauth_clients: self.include_oauth_clients,
        }
    }
}
