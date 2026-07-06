use sqlx::Row;
use tonic::Status;

use crate::grpc::pb::nvbes::billing::v1::{
    AdminMeterUsage, AdminTenantUsage, AdminUsageActionKind, AdminUsageActionRequest,
    AdminUsageActionResult, AdminUsageCenterSnapshot, AdminUsageCorrection, AdminUsageRollup,
};

pub async fn usage_center(db: &sqlx::PgPool) -> Result<AdminUsageCenterSnapshot, Status> {
    let metrics = sqlx::query(
        r#"
        SELECT
          (SELECT COUNT(*) FROM billing_meter_definitions WHERE status = 'active')
            AS active_meter_count,
          (
            SELECT COUNT(*) FROM billing_usage_events
            WHERE occurred_at >= NOW() - INTERVAL '24 hours'
          ) AS usage_event_count_24h,
          (
            SELECT COALESCE(SUM(quantity), 0) FROM billing_usage_events
            WHERE occurred_at >= NOW() - INTERVAL '24 hours'
          ) AS usage_quantity_24h,
          (
            SELECT COUNT(*) FROM billing_usage_corrections
            WHERE created_at >= NOW() - INTERVAL '30 days'
          ) AS correction_count_30d,
          (
            SELECT COUNT(*) FROM billing_usage_rollups
            WHERE period_end >= CURRENT_DATE
          ) AS rollup_count_current_period,
          (
            SELECT COUNT(DISTINCT tenant_id) FROM billing_usage_events
            WHERE occurred_at >= NOW() - INTERVAL '24 hours'
          ) AS distinct_tenant_count_24h
        "#,
    )
    .fetch_one(db)
    .await
    .map_err(crate::grpc::service_status::sql_status)?;

    Ok(AdminUsageCenterSnapshot {
        active_meter_count: metrics.get("active_meter_count"),
        usage_event_count_24h: metrics.get("usage_event_count_24h"),
        usage_quantity_24h: metrics.get("usage_quantity_24h"),
        correction_count_30d: metrics.get("correction_count_30d"),
        rollup_count_current_period: metrics.get("rollup_count_current_period"),
        distinct_tenant_count_24h: metrics.get("distinct_tenant_count_24h"),
        meter_usage_24h: load_meter_usage(db).await?,
        tenant_usage_24h: load_tenant_usage(db).await?,
        recent_rollups: load_recent_rollups(db).await?,
        recent_corrections: load_recent_corrections(db).await?,
    })
}

async fn load_meter_usage(db: &sqlx::PgPool) -> Result<Vec<AdminMeterUsage>, Status> {
    let rows = sqlx::query(
        r#"
        SELECT meter_code, unit, COUNT(*) AS event_count, COALESCE(SUM(quantity), 0) AS quantity
        FROM billing_usage_events
        WHERE occurred_at >= NOW() - INTERVAL '24 hours'
        GROUP BY meter_code, unit
        ORDER BY quantity DESC, event_count DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await
    .map_err(crate::grpc::service_status::sql_status)?;

    Ok(rows
        .into_iter()
        .map(|row| AdminMeterUsage {
            meter_code: row.get("meter_code"),
            unit: row.get("unit"),
            event_count: row.get("event_count"),
            quantity: row.get("quantity"),
        })
        .collect())
}

pub async fn run_usage_action(
    db: &sqlx::PgPool,
    tenant_id: uuid::Uuid,
    actor_principal_id: uuid::Uuid,
    request: AdminUsageActionRequest,
) -> Result<AdminUsageActionResult, Status> {
    validate_reason(&request.reason)?;
    match request.action_kind() {
        AdminUsageActionKind::CorrectUsage => {
            correct_usage(db, tenant_id, actor_principal_id, request).await
        }
        AdminUsageActionKind::FreezeMeter => freeze_meter(db, request).await,
        AdminUsageActionKind::ReplayRollup => replay_rollup(db, tenant_id, request).await,
        AdminUsageActionKind::Unspecified => Err(Status::invalid_argument(
            "admin usage action kind is required",
        )),
    }
}

async fn correct_usage(
    db: &sqlx::PgPool,
    tenant_id: uuid::Uuid,
    actor_principal_id: uuid::Uuid,
    request: AdminUsageActionRequest,
) -> Result<AdminUsageActionResult, Status> {
    validate_meter_code(&request.meter_code)?;
    if request.quantity_delta == 0 {
        return Err(Status::invalid_argument(
            "invalid_usage_correction: Usage correction quantity delta cannot be zero.",
        ));
    }
    let usage_event_id =
        crate::grpc::service_status::optional_uuid(&request.usage_event_id, "usage_event_id")?;
    let correction_id = sqlx::query_scalar::<_, uuid::Uuid>(
        "INSERT INTO billing_usage_corrections (
           tenant_id, usage_event_id, meter_code, quantity_delta, reason, created_by_principal_id
         ) VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
    )
    .bind(tenant_id)
    .bind(usage_event_id)
    .bind(&request.meter_code)
    .bind(request.quantity_delta)
    .bind(&request.reason)
    .bind(actor_principal_id)
    .fetch_one(db)
    .await
    .map_err(crate::grpc::service_status::sql_status)?;

    Ok(action_result(
        correction_id,
        "correct_usage",
        "applied",
        "usage.correction.created",
        "billing_usage_correction",
        request.meter_code,
        request.quantity_delta,
        serde_json::json!({ "usage_event_id": usage_event_id }),
        "Usage correction applied.",
    ))
}

async fn freeze_meter(
    db: &sqlx::PgPool,
    request: AdminUsageActionRequest,
) -> Result<AdminUsageActionResult, Status> {
    validate_meter_code(&request.meter_code)?;
    let meter_id = sqlx::query_scalar::<_, uuid::Uuid>(
        "UPDATE billing_meter_definitions
         SET status = 'frozen', updated_at = NOW()
         WHERE code = $1 AND status <> 'frozen'
         RETURNING id",
    )
    .bind(&request.meter_code)
    .fetch_optional(db)
    .await
    .map_err(crate::grpc::service_status::sql_status)?
    .ok_or_else(|| {
        Status::failed_precondition("meter_not_freezable: Meter is missing or already frozen.")
    })?;

    Ok(action_result(
        meter_id,
        "freeze_meter",
        "applied",
        "usage.meter.frozen",
        "billing_meter_definition",
        request.meter_code,
        0,
        serde_json::json!({ "meter_id": meter_id }),
        "Meter frozen by back-office.",
    ))
}

async fn replay_rollup(
    db: &sqlx::PgPool,
    tenant_id: uuid::Uuid,
    request: AdminUsageActionRequest,
) -> Result<AdminUsageActionResult, Status> {
    let rollup_id = crate::grpc::service_status::parse_uuid(&request.target_id, "target_id")?;
    let row = sqlx::query(
        "UPDATE billing_usage_rollups
         SET updated_at = NOW()
         WHERE id = $1 AND tenant_id = $2
         RETURNING id, meter_code",
    )
    .bind(rollup_id)
    .bind(tenant_id)
    .fetch_optional(db)
    .await
    .map_err(crate::grpc::service_status::sql_status)?
    .ok_or_else(|| Status::not_found("usage_rollup_not_found: Usage rollup not found."))?;
    let meter_code: String = row.get("meter_code");

    Ok(action_result(
        rollup_id,
        "replay_rollup",
        "replayed",
        "usage.rollup.replayed",
        "billing_usage_rollup",
        meter_code,
        0,
        serde_json::json!({ "rollup_id": rollup_id }),
        "Usage rollup replay requested.",
    ))
}

fn action_result(
    object_id: uuid::Uuid,
    action_kind: &str,
    status: &str,
    audit_action: &str,
    target_type: &str,
    meter_code: String,
    quantity_delta: i64,
    metadata: serde_json::Value,
    description: &str,
) -> AdminUsageActionResult {
    AdminUsageActionResult {
        object_id: object_id.to_string(),
        action_kind: action_kind.to_string(),
        status: status.to_string(),
        audit_action: audit_action.to_string(),
        target_type: target_type.to_string(),
        meter_code,
        quantity_delta,
        metadata_json: metadata.to_string(),
        description: description.to_string(),
    }
}

fn validate_meter_code(value: &str) -> Result<(), Status> {
    let len = value.trim().len();
    if (2..=96).contains(&len) {
        return Ok(());
    }
    Err(Status::invalid_argument(
        "invalid_usage_meter_code: Usage meter code must contain between 2 and 96 characters.",
    ))
}

fn validate_reason(value: &str) -> Result<(), Status> {
    let len = value.trim().len();
    if (8..=500).contains(&len) {
        return Ok(());
    }
    Err(Status::invalid_argument(
        "invalid_usage_reason: Usage action reason must contain between 8 and 500 characters.",
    ))
}

async fn load_tenant_usage(db: &sqlx::PgPool) -> Result<Vec<AdminTenantUsage>, Status> {
    let rows = sqlx::query(
        r#"
        SELECT ue.tenant_id, t.name AS tenant_name, COUNT(*) AS event_count,
          COALESCE(SUM(ue.quantity), 0) AS quantity
        FROM billing_usage_events ue
        JOIN tenants t ON t.id = ue.tenant_id
        WHERE ue.occurred_at >= NOW() - INTERVAL '24 hours'
        GROUP BY ue.tenant_id, t.name
        ORDER BY quantity DESC, event_count DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await
    .map_err(crate::grpc::service_status::sql_status)?;

    Ok(rows
        .into_iter()
        .map(|row| AdminTenantUsage {
            tenant_id: row.get::<uuid::Uuid, _>("tenant_id").to_string(),
            tenant_name: row.get("tenant_name"),
            event_count: row.get("event_count"),
            quantity: row.get("quantity"),
        })
        .collect())
}

async fn load_recent_rollups(db: &sqlx::PgPool) -> Result<Vec<AdminUsageRollup>, Status> {
    let rows = sqlx::query(
        r#"
        SELECT ur.id, ur.tenant_id, t.name AS tenant_name, ur.workspace_id,
          w.name AS workspace_name, ur.meter_code, ur.quantity, ur.unit,
          ur.period_start, ur.period_end
        FROM billing_usage_rollups ur
        JOIN tenants t ON t.id = ur.tenant_id
        LEFT JOIN workspaces w ON w.id = ur.workspace_id
        ORDER BY ur.updated_at DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await
    .map_err(crate::grpc::service_status::sql_status)?;

    Ok(rows
        .into_iter()
        .map(|row| AdminUsageRollup {
            id: row.get::<uuid::Uuid, _>("id").to_string(),
            tenant_id: row.get::<uuid::Uuid, _>("tenant_id").to_string(),
            tenant_name: row.get("tenant_name"),
            workspace_id: row
                .get::<Option<uuid::Uuid>, _>("workspace_id")
                .map(|value| value.to_string())
                .unwrap_or_default(),
            workspace_name: row
                .get::<Option<String>, _>("workspace_name")
                .unwrap_or_default(),
            meter_code: row.get("meter_code"),
            quantity: row.get("quantity"),
            unit: row.get("unit"),
            period_start: row.get::<chrono::NaiveDate, _>("period_start").to_string(),
            period_end: row.get::<chrono::NaiveDate, _>("period_end").to_string(),
        })
        .collect())
}

async fn load_recent_corrections(db: &sqlx::PgPool) -> Result<Vec<AdminUsageCorrection>, Status> {
    let rows = sqlx::query(
        r#"
        SELECT uc.id, uc.tenant_id, t.name AS tenant_name, uc.meter_code,
          uc.quantity_delta, uc.reason, uc.created_by_principal_id, uc.created_at
        FROM billing_usage_corrections uc
        JOIN tenants t ON t.id = uc.tenant_id
        ORDER BY uc.created_at DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await
    .map_err(crate::grpc::service_status::sql_status)?;

    Ok(rows
        .into_iter()
        .map(|row| AdminUsageCorrection {
            id: row.get::<uuid::Uuid, _>("id").to_string(),
            tenant_id: row.get::<uuid::Uuid, _>("tenant_id").to_string(),
            tenant_name: row.get("tenant_name"),
            meter_code: row.get("meter_code"),
            quantity_delta: row.get("quantity_delta"),
            reason: row.get("reason"),
            created_by_principal_id: row
                .get::<Option<uuid::Uuid>, _>("created_by_principal_id")
                .map(|value| value.to_string())
                .unwrap_or_default(),
            created_at: row
                .get::<chrono::DateTime<chrono::Utc>, _>("created_at")
                .to_rfc3339(),
        })
        .collect())
}
