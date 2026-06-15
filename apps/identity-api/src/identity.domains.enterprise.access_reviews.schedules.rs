use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::types::*;
use crate::http::error::AppError;

#[derive(Debug, FromRow)]
pub struct AccessReviewScheduleRow {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub include_members: bool,
    pub include_roles: bool,
    pub include_service_accounts: bool,
    pub include_oauth_clients: bool,
    pub recurrence_days: i32,
    pub due_after_days: i32,
    pub next_run_at: DateTime<Utc>,
    pub last_campaign_id: Option<Uuid>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub disabled_at: Option<DateTime<Utc>>,
}

impl AccessReviewScheduleRow {
    pub fn scope(&self) -> AccessReviewCampaignScopeInput {
        AccessReviewCampaignScopeInput {
            include_members: self.include_members,
            include_roles: self.include_roles,
            include_service_accounts: self.include_service_accounts,
            include_oauth_clients: self.include_oauth_clients,
        }
    }

    pub fn into_view(self) -> AccessReviewSchedule {
        let scope = self.scope();
        AccessReviewSchedule {
            id: self.id,
            name: self.name,
            description: self.description,
            recurrence_days: self.recurrence_days,
            due_after_days: self.due_after_days,
            next_run_at: self.next_run_at,
            last_campaign_id: self.last_campaign_id,
            created_by: self.created_by,
            created_at: self.created_at,
            disabled_at: self.disabled_at,
            scope,
        }
    }
}

#[derive(Debug, Default)]
pub struct AccessReviewScheduleRun {
    pub campaigns_created: u64,
    pub empty_schedules: u64,
}

pub async fn insert_schedule(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    created_by: Uuid,
    input: &CreateAccessReviewScheduleInput,
) -> Result<Uuid, AppError> {
    Ok(sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO access_review_schedules (
          tenant_id, name, description, include_members, include_roles,
          include_service_accounts, include_oauth_clients, recurrence_days,
          due_after_days, created_by
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        RETURNING id
        "#,
    )
    .bind(tenant_id)
    .bind(input.name.trim())
    .bind(input.description.as_deref().map(str::trim))
    .bind(input.scope.include_members)
    .bind(input.scope.include_roles)
    .bind(input.scope.include_service_accounts)
    .bind(input.scope.include_oauth_clients)
    .bind(input.recurrence_days)
    .bind(input.due_after_days)
    .bind(created_by)
    .fetch_one(&mut **tx)
    .await?)
}

pub async fn list_schedules(
    db: &PgPool,
    tenant_id: Uuid,
) -> Result<Vec<AccessReviewScheduleRow>, AppError> {
    Ok(sqlx::query_as::<_, AccessReviewScheduleRow>(
        r#"
        SELECT id, name, description, include_members, include_roles,
          include_service_accounts, include_oauth_clients, recurrence_days,
          due_after_days, next_run_at, last_campaign_id, created_by, created_at,
          disabled_at
        FROM access_review_schedules
        WHERE tenant_id = $1
        ORDER BY disabled_at ASC NULLS FIRST, next_run_at ASC, created_at DESC
        "#,
    )
    .bind(tenant_id)
    .fetch_all(db)
    .await?)
}

pub async fn get_schedule(
    db: &PgPool,
    tenant_id: Uuid,
    schedule_id: Uuid,
) -> Result<Option<AccessReviewScheduleRow>, AppError> {
    Ok(sqlx::query_as::<_, AccessReviewScheduleRow>(
        r#"
        SELECT id, name, description, include_members, include_roles,
          include_service_accounts, include_oauth_clients, recurrence_days,
          due_after_days, next_run_at, last_campaign_id, created_by, created_at,
          disabled_at
        FROM access_review_schedules
        WHERE tenant_id = $1 AND id = $2
        "#,
    )
    .bind(tenant_id)
    .bind(schedule_id)
    .fetch_optional(db)
    .await?)
}
