use chrono::{DateTime, Duration, Utc};
use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::scheduler_types::{
    AccessReviewCampaignScopeInput, AccessReviewScheduleRun, ScheduleExecution,
    ScheduleExecutionRow,
};
use crate::{AccountError, AccountResult};

pub(crate) async fn materialize_due_schedules(
    db: &PgPool,
    limit: i64,
) -> AccountResult<AccessReviewScheduleRun> {
    let mut tx = db.begin().await.map_err(AccountError::from)?;
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

    tx.commit().await.map_err(AccountError::from)?;
    Ok(run)
}

async fn claim_due_schedules(
    tx: &mut Transaction<'_, Postgres>,
    limit: i64,
) -> AccountResult<Vec<ScheduleExecutionRow>> {
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
    .await
    .map_err(AccountError::from)?)
}

pub(crate) async fn run_schedule_now(
    db: &PgPool,
    tenant_id: Uuid,
    actor_id: Uuid,
    schedule_id: Uuid,
) -> AccountResult<Uuid> {
    let mut tx = db.begin().await.map_err(AccountError::from)?;
    let schedule = get_schedule_for_update(&mut tx, tenant_id, schedule_id)
        .await?
        .ok_or_else(|| {
            AccountError::not_found(
                "access_review_schedule_not_found",
                "Access review schedule not found.",
            )
        })?;

    if schedule.disabled_at.is_some() {
        return Err(AccountError::conflict(
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
            tx.commit().await.map_err(AccountError::from)?;
            Ok(campaign_id)
        }
        ScheduleExecution::Empty => Err(AccountError::bad_request(
            "empty_access_review_scope",
            "The selected access review scope does not contain any reviewable access.",
        )),
    }
}

async fn get_schedule_for_update(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    schedule_id: Uuid,
) -> AccountResult<Option<ScheduleExecutionRow>> {
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
    .await
    .map_err(AccountError::from)?)
}

async fn execute_schedule(
    tx: &mut Transaction<'_, Postgres>,
    schedule: &ScheduleExecutionRow,
    created_by: Uuid,
    now: DateTime<Utc>,
) -> AccountResult<ScheduleExecution> {
    let due_at = now + Duration::days(schedule.due_after_days.into());
    let campaign_id = insert_campaign(
        tx,
        schedule.tenant_id,
        created_by,
        &schedule.name,
        schedule.description.as_deref(),
        due_at,
    )
    .await?;
    let item_count =
        insert_snapshot_items(tx, campaign_id, schedule.tenant_id, &schedule.scope()).await?;
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

async fn insert_campaign(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    created_by: Uuid,
    name: &str,
    description: Option<&str>,
    due_at: DateTime<Utc>,
) -> AccountResult<Uuid> {
    Ok(sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO access_review_campaigns (tenant_id, name, description, due_at, created_by)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id
        "#,
    )
    .bind(tenant_id)
    .bind(name)
    .bind(description)
    .bind(due_at)
    .bind(created_by)
    .fetch_one(&mut **tx)
    .await
    .map_err(AccountError::from)?)
}

async fn insert_snapshot_items(
    tx: &mut Transaction<'_, Postgres>,
    campaign_id: Uuid,
    tenant_id: Uuid,
    scope: &AccessReviewCampaignScopeInput,
) -> AccountResult<u64> {
    let result = sqlx::query(
        r#"
        INSERT INTO access_review_items (
          campaign_id, tenant_id, item_type, subject_id, subject_label,
          workspace_id, role, status, evidence
        )
        SELECT $1, $2, snapshot.item_type::access_review_item_type, snapshot.subject_id,
          snapshot.subject_label, snapshot.workspace_id, snapshot.role, snapshot.status,
          snapshot.evidence
        FROM (
          SELECT 'member' AS item_type, tm.principal_id::text AS subject_id,
            COALESCE(u.email, p.display_name) AS subject_label, NULL::uuid AS workspace_id,
            tm.role::text AS role, tm.status::text AS status,
            jsonb_build_object('principal_kind', tm.principal_kind::text, 'created_at', tm.created_at) AS evidence
          FROM tenant_memberships tm
          INNER JOIN principals p ON p.id = tm.principal_id
          LEFT JOIN users u ON u.principal_id = tm.principal_id
          WHERE tm.tenant_id = $2 AND $3

          UNION ALL

          SELECT 'role' AS item_type, concat(wm.principal_id::text, ':', wm.workspace_id::text) AS subject_id,
            concat(COALESCE(u.email, p.display_name), ' in ', w.name) AS subject_label,
            wm.workspace_id, wm.role::text AS role, wm.status::text AS status,
            jsonb_build_object('workspace_name', w.name, 'source', wm.source::text, 'created_at', wm.created_at) AS evidence
          FROM workspace_memberships wm
          INNER JOIN workspaces w ON w.id = wm.workspace_id
          INNER JOIN principals p ON p.id = wm.principal_id
          LEFT JOIN users u ON u.principal_id = wm.principal_id
          WHERE w.tenant_id = $2 AND wm.status IN ('active', 'suspended') AND $4

          UNION ALL

          SELECT 'service_account' AS item_type, sa.principal_id::text AS subject_id,
            sa.name AS subject_label, sa.workspace_id, wm.role::text AS role,
            p.status::text AS status,
            jsonb_build_object('auth_method', sa.auth_method, 'client_id', sa.client_id, 'created_at', sa.created_at) AS evidence
          FROM service_accounts sa
          INNER JOIN principals p ON p.id = sa.principal_id
          LEFT JOIN workspace_memberships wm ON wm.principal_id = sa.principal_id AND wm.workspace_id = sa.workspace_id
          WHERE sa.tenant_id = $2 AND $5

          UNION ALL

          SELECT 'oauth_client' AS item_type, oc.client_id AS subject_id,
            oc.name AS subject_label, NULL::uuid AS workspace_id, NULL::text AS role,
            CASE WHEN oc.revoked_at IS NULL THEN 'active' ELSE 'revoked' END AS status,
            jsonb_build_object(
              'client_type', oc.client_type::text,
              'owner_scope_type', oc.owner_scope_type::text,
              'owner_scope_id', oc.owner_scope_id,
              'last_used_at', oc.last_used_at,
              'requires_admin_consent', oc.requires_admin_consent
            ) AS evidence
          FROM oauth_clients oc
          WHERE oc.tenant_id = $2 AND $6
        ) snapshot
        "#,
    )
    .bind(campaign_id)
    .bind(tenant_id)
    .bind(scope.include_members)
    .bind(scope.include_roles)
    .bind(scope.include_service_accounts)
    .bind(scope.include_oauth_clients)
    .execute(&mut **tx)
    .await
    .map_err(AccountError::from)?;
    Ok(result.rows_affected())
}

async fn update_after_run(
    tx: &mut Transaction<'_, Postgres>,
    schedule_id: Uuid,
    campaign_id: Option<Uuid>,
    next_run_at: DateTime<Utc>,
) -> AccountResult<()> {
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
    .map_err(AccountError::from)?;
    Ok(())
}

async fn delete_empty_campaign(
    tx: &mut Transaction<'_, Postgres>,
    campaign_id: Uuid,
) -> AccountResult<()> {
    sqlx::query("DELETE FROM access_review_campaigns WHERE id = $1")
        .bind(campaign_id)
        .execute(&mut **tx)
        .await
        .map_err(AccountError::from)?;
    Ok(())
}

async fn audit_created_campaign(
    tx: &mut Transaction<'_, Postgres>,
    schedule: &ScheduleExecutionRow,
    actor_id: Uuid,
    campaign_id: Uuid,
    item_count: u64,
    due_at: DateTime<Utc>,
) -> AccountResult<()> {
    insert_audit(
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

async fn insert_audit(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    actor_id: Uuid,
    action: &'static str,
    target_type: &'static str,
    target_id: Option<Uuid>,
    metadata: Value,
) -> AccountResult<()> {
    sqlx::query(
        r#"
        INSERT INTO audit_events (
          tenant_id,
          workspace_id,
          actor_principal_id,
          action,
          target_type,
          target_id,
          ip,
          user_agent,
          metadata
        )
        VALUES ($1, NULL, $2, $3, $4, $5, NULL, NULL, $6)
        "#,
    )
    .bind(tenant_id)
    .bind(actor_id)
    .bind(action)
    .bind(target_type)
    .bind(target_id)
    .bind(sqlx::types::Json(metadata))
    .execute(&mut **tx)
    .await
    .map_err(AccountError::from)?;
    Ok(())
}

fn next_run_at(now: DateTime<Utc>, recurrence_days: i32) -> DateTime<Utc> {
    now + Duration::days(recurrence_days.into())
}
