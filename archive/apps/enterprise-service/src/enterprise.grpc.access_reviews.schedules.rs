use chrono::{DateTime, Days, Utc};
use sqlx::{Row, postgres::PgRow};
use tonic::Status;
use uuid::Uuid;

#[path = "enterprise.grpc.access_reviews.schedules.runtime.rs"]
mod runtime;

use super::{AccessReviewScope, insert_snapshot_items};
use crate::grpc::{
    pb::nvbes::enterprise::v1 as enterprise,
    service_status::{non_empty, sql_status},
};

pub use runtime::materialize_due_schedules;

pub async fn list_access_review_schedules(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
) -> Result<enterprise::ListAccessReviewSchedulesResponse, Status> {
    let schedules = sqlx::query(&schedule_select_sql("WHERE tenant_id = $1"))
        .bind(tenant_id)
        .fetch_all(db)
        .await
        .map_err(sql_status)?
        .into_iter()
        .map(schedule_from_row)
        .collect();

    Ok(enterprise::ListAccessReviewSchedulesResponse { schedules })
}

pub async fn create_access_review_schedule(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    actor_id: Uuid,
    request: enterprise::CreateAccessReviewScheduleRequest,
) -> Result<enterprise::AccessReviewSchedule, Status> {
    let scope = scope_from_request(&request)?;
    validate_schedule_bounds(request.recurrence_days, request.due_after_days)?;
    let name = non_empty(request.name, "name")?;
    let description = optional_text(request.description);

    let row = sqlx::query(schedule_insert_sql())
        .bind(tenant_id)
        .bind(name)
        .bind(description)
        .bind(scope.include_members)
        .bind(scope.include_roles)
        .bind(scope.include_service_accounts)
        .bind(scope.include_oauth_clients)
        .bind(request.recurrence_days)
        .bind(request.due_after_days)
        .bind(actor_id)
        .fetch_one(db)
        .await
        .map_err(sql_status)?;

    Ok(schedule_from_row(row))
}

pub async fn set_access_review_schedule_enabled(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    schedule_id: Uuid,
    enabled: bool,
) -> Result<enterprise::AccessReviewSchedule, Status> {
    let row = sqlx::query(&format!(
        r#"
        UPDATE access_review_schedules
        SET disabled_at = CASE WHEN $3 THEN NULL ELSE COALESCE(disabled_at, NOW()) END
        WHERE tenant_id = $1 AND id = $2
        RETURNING {}
        "#,
        schedule_columns()
    ))
    .bind(tenant_id)
    .bind(schedule_id)
    .bind(enabled)
    .fetch_optional(db)
    .await
    .map_err(sql_status)?
    .ok_or_else(|| Status::not_found("enterprise access review schedule was not found"))?;

    Ok(schedule_from_row(row))
}

pub async fn run_access_review_schedule(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    schedule_id: Uuid,
    actor_id: Uuid,
) -> Result<enterprise::AccessReview, Status> {
    let mut tx = db.begin().await.map_err(sql_status)?;
    let schedule = sqlx::query(&schedule_select_sql("WHERE tenant_id = $1 AND id = $2"))
        .bind(tenant_id)
        .bind(schedule_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(sql_status)?
        .ok_or_else(|| Status::not_found("enterprise access review schedule was not found"))?;
    if schedule
        .get::<Option<DateTime<Utc>>, _>("disabled_at")
        .is_some()
    {
        return Err(Status::failed_precondition(
            "disabled access review schedules cannot be run",
        ));
    }

    let scope = scope_from_row(&schedule);
    let due_after_days = schedule.get::<i32, _>("due_after_days");
    let recurrence_days = schedule.get::<i32, _>("recurrence_days");
    let due_at = Utc::now()
        .checked_add_days(Days::new(due_after_days as u64))
        .ok_or_else(|| Status::invalid_argument("due_after_days is out of range"))?;
    let campaign_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO access_review_campaigns (tenant_id, name, description, due_at, created_by)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id
        "#,
    )
    .bind(tenant_id)
    .bind(schedule.get::<String, _>("name"))
    .bind(schedule.get::<Option<String>, _>("description"))
    .bind(due_at)
    .bind(actor_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(sql_status)?;

    let item_count = insert_snapshot_items(&mut tx, campaign_id, tenant_id, &scope).await?;
    if item_count == 0 {
        return Err(Status::failed_precondition(
            "access review schedule scope did not contain reviewable access",
        ));
    }
    let next_run_at = Utc::now()
        .checked_add_days(Days::new(recurrence_days as u64))
        .ok_or_else(|| Status::invalid_argument("recurrence_days is out of range"))?;
    sqlx::query(
        r#"
        UPDATE access_review_schedules
        SET last_campaign_id = $3, next_run_at = $4
        WHERE tenant_id = $1 AND id = $2
        "#,
    )
    .bind(tenant_id)
    .bind(schedule_id)
    .bind(campaign_id)
    .bind(next_run_at)
    .execute(&mut *tx)
    .await
    .map_err(sql_status)?;
    tx.commit().await.map_err(sql_status)?;

    Ok(enterprise::AccessReview {
        review_id: campaign_id.to_string(),
        tenant_id: tenant_id.to_string(),
        scope: scope.as_contract_scope().to_string(),
        status: "active".to_string(),
        due_at: due_at.to_rfc3339(),
    })
}

fn schedule_from_row(row: PgRow) -> enterprise::AccessReviewSchedule {
    let scope = scope_from_row(&row);
    enterprise::AccessReviewSchedule {
        schedule_id: row.get::<Uuid, _>("id").to_string(),
        tenant_id: row.get::<Uuid, _>("tenant_id").to_string(),
        name: row.get("name"),
        description: row
            .get::<Option<String>, _>("description")
            .unwrap_or_default(),
        scope: scope.as_contract_scope().to_string(),
        include_members: scope.include_members,
        include_roles: scope.include_roles,
        include_service_accounts: scope.include_service_accounts,
        include_oauth_clients: scope.include_oauth_clients,
        recurrence_days: row.get("recurrence_days"),
        due_after_days: row.get("due_after_days"),
        next_run_at: time_string(row.get("next_run_at")),
        last_review_id: row
            .get::<Option<Uuid>, _>("last_campaign_id")
            .map(|id| id.to_string())
            .unwrap_or_default(),
        created_by: row.get::<Uuid, _>("created_by").to_string(),
        created_at: time_string(row.get("created_at")),
        disabled_at: row
            .get::<Option<DateTime<Utc>>, _>("disabled_at")
            .map(time_string)
            .unwrap_or_default(),
    }
}

fn scope_from_row(row: &PgRow) -> AccessReviewScope {
    AccessReviewScope {
        include_members: row.get("include_members"),
        include_roles: row.get("include_roles"),
        include_service_accounts: row.get("include_service_accounts"),
        include_oauth_clients: row.get("include_oauth_clients"),
    }
}

fn scope_from_request(
    request: &enterprise::CreateAccessReviewScheduleRequest,
) -> Result<AccessReviewScope, Status> {
    if request.include_members
        || request.include_roles
        || request.include_service_accounts
        || request.include_oauth_clients
    {
        Ok(AccessReviewScope {
            include_members: request.include_members,
            include_roles: request.include_roles,
            include_service_accounts: request.include_service_accounts,
            include_oauth_clients: request.include_oauth_clients,
        })
    } else {
        AccessReviewScope::parse(&request.scope)
    }
}

fn schedule_select_sql(predicate: &str) -> String {
    format!(
        r#"
        SELECT {}
        FROM access_review_schedules
        {predicate}
        ORDER BY disabled_at ASC NULLS FIRST, next_run_at ASC, created_at DESC
        "#,
        schedule_columns()
    )
}

fn schedule_insert_sql() -> &'static str {
    r#"
    INSERT INTO access_review_schedules (
      tenant_id, name, description, include_members, include_roles,
      include_service_accounts, include_oauth_clients, recurrence_days,
      due_after_days, created_by
    )
    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
    RETURNING
      id, tenant_id, name, description, include_members, include_roles,
      include_service_accounts, include_oauth_clients, recurrence_days,
      due_after_days, next_run_at, last_campaign_id, created_by, created_at,
      disabled_at
    "#
}

fn schedule_columns() -> &'static str {
    r#"
      id, tenant_id, name, description, include_members, include_roles,
      include_service_accounts, include_oauth_clients, recurrence_days,
      due_after_days, next_run_at, last_campaign_id, created_by, created_at,
      disabled_at
    "#
}

fn validate_schedule_bounds(recurrence_days: i32, due_after_days: i32) -> Result<(), Status> {
    if !(7..=366).contains(&recurrence_days) {
        return Err(Status::invalid_argument(
            "recurrence_days must be between 7 and 366",
        ));
    }
    if due_after_days < 1 || due_after_days > recurrence_days {
        return Err(Status::invalid_argument(
            "due_after_days must be between 1 and recurrence_days",
        ));
    }
    Ok(())
}

fn optional_text(value: String) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn time_string(value: DateTime<Utc>) -> String {
    value.to_rfc3339()
}
