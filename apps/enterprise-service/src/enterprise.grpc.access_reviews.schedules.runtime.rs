use chrono::{DateTime, Days, Utc};
use sqlx::{Postgres, Row, Transaction, postgres::PgRow};
use tonic::Status;
use uuid::Uuid;

use crate::grpc::{
    pb::nvbes::enterprise::v1 as enterprise,
    service_status::{non_empty, sql_status},
};

const DEFAULT_DUE_SCHEDULE_LIMIT: i64 = 25;
const MAX_DUE_SCHEDULE_LIMIT: i64 = 100;

pub async fn materialize_due_schedules(
    db: &sqlx::PgPool,
    request: enterprise::MaterializeDueAccessReviewSchedulesRequest,
) -> Result<enterprise::AccessReviewScheduleRun, Status> {
    let limit = requested_limit(request.limit);
    let mut tx = db.begin().await.map_err(sql_status)?;
    let schedules = claim_due_schedules(&mut tx, limit).await?;
    let mut campaigns_created = 0;
    let mut empty_schedules = 0;

    for schedule in schedules {
        let execution = execute_schedule(&mut tx, &schedule).await?;
        let next_run_at = next_run_at(
            Utc::now(),
            schedule.get::<i32, _>("recurrence_days"),
            "recurrence_days",
        )?;
        update_after_run(
            &mut tx,
            schedule.get("id"),
            execution.campaign_id,
            next_run_at,
        )
        .await?;
        if let Some(campaign_id) = execution.campaign_id {
            insert_schedule_audit(&mut tx, &schedule, campaign_id, execution).await?;
            campaigns_created += 1;
        } else {
            empty_schedules += 1;
        }
    }

    tx.commit().await.map_err(sql_status)?;
    Ok(enterprise::AccessReviewScheduleRun {
        campaigns_created,
        empty_schedules,
    })
}

async fn claim_due_schedules(
    tx: &mut Transaction<'_, Postgres>,
    limit: i64,
) -> Result<Vec<PgRow>, Status> {
    sqlx::query(&format!(
        r#"
        SELECT {}
        FROM access_review_schedules
        WHERE disabled_at IS NULL AND next_run_at <= NOW()
        ORDER BY next_run_at ASC, created_at ASC
        LIMIT $1
        FOR UPDATE SKIP LOCKED
        "#,
        super::schedule_columns()
    ))
    .bind(limit)
    .fetch_all(&mut **tx)
    .await
    .map_err(sql_status)
}

async fn execute_schedule(
    tx: &mut Transaction<'_, Postgres>,
    schedule: &PgRow,
) -> Result<ScheduleExecution, Status> {
    let due_at = next_run_at(
        Utc::now(),
        schedule.get::<i32, _>("due_after_days"),
        "due_after_days",
    )?;
    let campaign_id = insert_campaign(tx, schedule, due_at).await?;
    let scope = super::scope_from_row(schedule);
    let item_count = super::super::insert_snapshot_items(
        tx,
        campaign_id,
        schedule.get::<Uuid, _>("tenant_id"),
        &scope,
    )
    .await?;
    if item_count == 0 {
        delete_empty_campaign(tx, campaign_id).await?;
        return Ok(ScheduleExecution {
            campaign_id: None,
            due_at,
            item_count,
        });
    }
    Ok(ScheduleExecution {
        campaign_id: Some(campaign_id),
        due_at,
        item_count,
    })
}

async fn insert_campaign(
    tx: &mut Transaction<'_, Postgres>,
    schedule: &PgRow,
    due_at: DateTime<Utc>,
) -> Result<Uuid, Status> {
    let name = non_empty(schedule.get::<String, _>("name"), "name")?;
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO access_review_campaigns (tenant_id, name, description, due_at, created_by)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id
        "#,
    )
    .bind(schedule.get::<Uuid, _>("tenant_id"))
    .bind(name)
    .bind(schedule.get::<Option<String>, _>("description"))
    .bind(due_at)
    .bind(schedule.get::<Uuid, _>("created_by"))
    .fetch_one(&mut **tx)
    .await
    .map_err(sql_status)
}

async fn update_after_run(
    tx: &mut Transaction<'_, Postgres>,
    schedule_id: Uuid,
    campaign_id: Option<Uuid>,
    next_run_at: DateTime<Utc>,
) -> Result<(), Status> {
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
    .await
    .map_err(sql_status)?;
    Ok(())
}

async fn delete_empty_campaign(
    tx: &mut Transaction<'_, Postgres>,
    campaign_id: Uuid,
) -> Result<(), Status> {
    sqlx::query("DELETE FROM access_review_campaigns WHERE id = $1")
        .bind(campaign_id)
        .execute(&mut **tx)
        .await
        .map_err(sql_status)?;
    Ok(())
}

async fn insert_schedule_audit(
    tx: &mut Transaction<'_, Postgres>,
    schedule: &PgRow,
    campaign_id: Uuid,
    execution: ScheduleExecution,
) -> Result<(), Status> {
    sqlx::query(
        r#"
        INSERT INTO audit_events (
          tenant_id, actor_principal_id, action, target_type, target_id,
          metadata, event_hash, created_at
        )
        VALUES (
          $1, $2, 'enterprise.access_review_campaign.created',
          'access_review_campaign', $3, $4, gen_random_uuid()::text, NOW()
        )
        "#,
    )
    .bind(schedule.get::<Uuid, _>("tenant_id"))
    .bind(schedule.get::<Uuid, _>("created_by"))
    .bind(campaign_id)
    .bind(serde_json::json!({
        "name": schedule.get::<String, _>("name"),
        "due_at": execution.due_at,
        "item_count": execution.item_count,
        "schedule_id": schedule.get::<Uuid, _>("id")
    }))
    .execute(&mut **tx)
    .await
    .map_err(sql_status)?;
    Ok(())
}

fn requested_limit(limit: i32) -> i64 {
    if limit <= 0 {
        DEFAULT_DUE_SCHEDULE_LIMIT
    } else {
        i64::from(limit).min(MAX_DUE_SCHEDULE_LIMIT)
    }
}

fn next_run_at(
    now: DateTime<Utc>,
    days: i32,
    field: &'static str,
) -> Result<DateTime<Utc>, Status> {
    if days <= 0 {
        return Err(Status::invalid_argument(format!(
            "{field} must be positive"
        )));
    }
    now.checked_add_days(Days::new(days as u64))
        .ok_or_else(|| Status::invalid_argument(format!("{field} is out of range")))
}

#[derive(Clone, Copy)]
struct ScheduleExecution {
    campaign_id: Option<Uuid>,
    due_at: DateTime<Utc>,
    item_count: u64,
}
