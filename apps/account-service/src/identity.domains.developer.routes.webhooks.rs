use axum::{
    Extension, Json,
    extract::{Path, State},
};
use uuid::Uuid;

use crate::{
    app::AppState,
    domains::developer::{
        service,
        types::{
            DeveloperConsoleLogsResponse, DeveloperWebhookDeliveriesResponse,
            DeveloperWebhookDeliverySummary, DeveloperWebhookEndpointsResponse,
        },
        webhooks_delivery,
    },
    http::{error::AppError, middleware::jwt::AuthContext},
};

pub async fn list_webhooks(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<DeveloperWebhookEndpointsResponse>, AppError> {
    let tenant_id = service::require_tenant_id(&auth)?;
    let webhooks = sqlx::query_as(
        r#"
        SELECT
          e.id,
          e.name,
          e.url,
          e.status::text AS status,
          COUNT(d.id) FILTER (WHERE d.status = 'failed')::bigint AS failed_delivery_count,
          e.created_at,
          e.updated_at
        FROM developer_webhook_endpoints e
        LEFT JOIN developer_webhook_deliveries d
          ON d.tenant_id = e.tenant_id
         AND d.endpoint_id = e.id
        WHERE e.tenant_id = $1
          AND e.revoked_at IS NULL
        GROUP BY e.id, e.name, e.url, e.status, e.created_at, e.updated_at
        ORDER BY e.updated_at DESC
        "#,
    )
    .bind(tenant_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(DeveloperWebhookEndpointsResponse { webhooks }))
}

pub async fn list_deliveries(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(endpoint_id): Path<Uuid>,
) -> Result<Json<DeveloperWebhookDeliveriesResponse>, AppError> {
    let tenant_id = service::require_tenant_id(&auth)?;
    let deliveries = list_delivery_summaries(&state.db, tenant_id, endpoint_id).await?;
    Ok(Json(DeveloperWebhookDeliveriesResponse { deliveries }))
}

pub async fn replay_delivery(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(delivery_id): Path<Uuid>,
) -> Result<Json<DeveloperWebhookDeliverySummary>, AppError> {
    let tenant_id = service::require_tenant_id(&auth)?;
    let original = find_replayable_delivery(&state.db, tenant_id, delivery_id).await?;

    let replayed = sqlx::query_as(
        r#"
        INSERT INTO developer_webhook_deliveries (
          endpoint_id,
          tenant_id,
          event_id,
          event_type,
          status,
          attempt_count,
          replayed_from_delivery_id
        )
        VALUES ($1, $2, $3, $4, 'pending', 0, $5)
        ON CONFLICT (tenant_id, replayed_from_delivery_id)
          WHERE replayed_from_delivery_id IS NOT NULL
        DO UPDATE SET
          replayed_from_delivery_id = developer_webhook_deliveries.replayed_from_delivery_id
        RETURNING
          id,
          endpoint_id,
          event_id,
          event_type,
          status::text AS status,
          attempt_count,
          response_status,
          error_message,
          created_at,
          delivered_at,
          replayed_from_delivery_id
        "#,
    )
    .bind(original.endpoint_id)
    .bind(tenant_id)
    .bind(original.event_id)
    .bind(&original.event_type)
    .bind(original.id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(replayed))
}

pub async fn list_logs(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<DeveloperConsoleLogsResponse>, AppError> {
    let tenant_id = service::require_tenant_id(&auth)?;
    let logs = sqlx::query_as(
        r#"
        SELECT
          d.id,
          'webhook' AS source,
          d.event_type,
          CASE WHEN d.status = 'failed' THEN 'error' ELSE 'info' END AS severity,
          COALESCE(d.error_message, d.status::text) AS message,
          d.created_at
        FROM developer_webhook_deliveries d
        WHERE d.tenant_id = $1
        ORDER BY d.created_at DESC
        LIMIT 200
        "#,
    )
    .bind(tenant_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(DeveloperConsoleLogsResponse { logs }))
}

async fn find_replayable_delivery(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    delivery_id: Uuid,
) -> Result<DeveloperWebhookDeliverySummary, AppError> {
    let delivery: DeveloperWebhookDeliverySummary = sqlx::query_as(
        r#"
        SELECT
          d.id,
          d.endpoint_id,
          d.event_id,
          d.event_type,
          d.status::text AS status,
          d.attempt_count,
          d.response_status,
          d.error_message,
          d.created_at,
          d.delivered_at,
          d.replayed_from_delivery_id
        FROM developer_webhook_deliveries d
        INNER JOIN developer_webhook_endpoints e
          ON e.tenant_id = d.tenant_id
         AND e.id = d.endpoint_id
        WHERE d.tenant_id = $1
          AND d.id = $2
          AND e.revoked_at IS NULL
        "#,
    )
    .bind(tenant_id)
    .bind(delivery_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| {
        AppError::not_found("webhook_delivery_not_found", "Webhook delivery not found")
    })?;

    webhooks_delivery::ensure_replayable_delivery_status(&delivery.status)?;

    Ok(delivery)
}

async fn list_delivery_summaries(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    endpoint_id: Uuid,
) -> Result<Vec<DeveloperWebhookDeliverySummary>, AppError> {
    sqlx::query_as(
        r#"
        SELECT
          id,
          endpoint_id,
          event_id,
          event_type,
          status::text AS status,
          attempt_count,
          response_status,
          error_message,
          created_at,
          delivered_at,
          replayed_from_delivery_id
        FROM developer_webhook_deliveries
        WHERE tenant_id = $1
          AND endpoint_id = $2
        ORDER BY created_at DESC
        LIMIT 100
        "#,
    )
    .bind(tenant_id)
    .bind(endpoint_id)
    .fetch_all(db)
    .await
    .map_err(AppError::from)
}
