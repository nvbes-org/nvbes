#[path = "developer.grpc.webhooks.persistence.rs"]
mod persistence;
#[path = "developer.grpc.webhooks.rows.rs"]
mod rows;
#[path = "developer.grpc.webhooks.validation.rs"]
mod validation;

use persistence::{current_events, insert_subscriptions};
use rows::{api_log_from_row, delivery_from_row, endpoint_from_row, endpoint_with_events};
use sqlx::Row;
use tonic::Status;
use uuid::Uuid;
use validation::{
    ensure_replayable, last4, optional_non_empty, optional_url, validate_events, validate_url,
};

use crate::grpc::{
    pb::nvbes::developer::v1 as developer,
    service_status::{non_empty, parse_uuid, sql_status},
};

pub async fn list_webhook_endpoints(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
) -> Result<developer::ListWebhookEndpointsResponse, Status> {
    let endpoints = sqlx::query(
        r#"
        SELECT
          endpoint.id,
          endpoint.tenant_id,
          endpoint.name,
          endpoint.url,
          endpoint.status::text AS status,
          endpoint.signing_secret_last4,
          endpoint.created_at,
          endpoint.updated_at,
          COUNT(DISTINCT delivery.id) FILTER (WHERE delivery.status = 'failed')::bigint
            AS failed_delivery_count,
          COALESCE(
            array_agg(subscription.event_type ORDER BY subscription.event_type)
              FILTER (WHERE subscription.event_type IS NOT NULL),
            ARRAY[]::text[]
          ) AS event_types
        FROM developer_webhook_endpoints endpoint
        LEFT JOIN developer_webhook_subscriptions subscription
          ON subscription.endpoint_id = endpoint.id
        LEFT JOIN developer_webhook_deliveries delivery
          ON delivery.tenant_id = endpoint.tenant_id
         AND delivery.endpoint_id = endpoint.id
        WHERE endpoint.tenant_id = $1
          AND endpoint.revoked_at IS NULL
        GROUP BY endpoint.id
        ORDER BY endpoint.updated_at DESC
        "#,
    )
    .bind(tenant_id)
    .fetch_all(db)
    .await
    .map_err(sql_status)?
    .into_iter()
    .map(|row| endpoint_from_row(row, String::new()))
    .collect();

    Ok(developer::ListWebhookEndpointsResponse {
        endpoints,
        page: None,
    })
}

pub async fn create_webhook_endpoint(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    actor_id: Uuid,
    request: developer::CreateWebhookEndpointRequest,
) -> Result<developer::WebhookEndpoint, Status> {
    let name = non_empty(request.name, "name")?;
    let url = validate_url(request.url)?;
    let events = validate_events(request.event_types)?;
    let signing_secret = format!("whsec_{}", Uuid::new_v4().simple());
    let signing_secret_last4 = last4(&signing_secret);

    let mut tx = db.begin().await.map_err(sql_status)?;
    let endpoint = sqlx::query(
        r#"
        INSERT INTO developer_webhook_endpoints (
          tenant_id,
          name,
          url,
          signing_secret_ciphertext,
          signing_secret_last4,
          created_by
        )
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING
          id,
          tenant_id,
          name,
          url,
          status::text AS status,
          signing_secret_last4,
          created_at,
          updated_at,
          0::bigint AS failed_delivery_count
        "#,
    )
    .bind(tenant_id)
    .bind(name)
    .bind(url)
    .bind(&signing_secret)
    .bind(&signing_secret_last4)
    .bind(actor_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(sql_status)?;

    insert_subscriptions(&mut tx, endpoint.get("id"), &events).await?;
    tx.commit().await.map_err(sql_status)?;

    Ok(endpoint_with_events(endpoint, events, signing_secret))
}

pub async fn update_webhook_endpoint(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    request: developer::UpdateWebhookEndpointRequest,
) -> Result<developer::WebhookEndpoint, Status> {
    let endpoint_id = parse_uuid(&request.endpoint_id, "endpoint_id")?;
    let current_events = current_events(db, tenant_id, endpoint_id).await?;
    let events = if request.event_types.is_empty() {
        current_events
    } else {
        validate_events(request.event_types)?
    };
    let name = optional_non_empty(request.name);
    let url = optional_url(request.url)?;
    let status = optional_non_empty(request.status);

    let mut tx = db.begin().await.map_err(sql_status)?;
    let endpoint = sqlx::query(
        r#"
        UPDATE developer_webhook_endpoints
        SET name = COALESCE($3, name),
            url = COALESCE($4, url),
            status = COALESCE($5, status::text)::developer_webhook_status,
            updated_at = NOW()
        WHERE id = $1
          AND tenant_id = $2
          AND revoked_at IS NULL
        RETURNING
          id,
          tenant_id,
          name,
          url,
          status::text AS status,
          signing_secret_last4,
          created_at,
          updated_at,
          (
            SELECT COUNT(delivery.id)
            FROM developer_webhook_deliveries delivery
            WHERE delivery.tenant_id = developer_webhook_endpoints.tenant_id
              AND delivery.endpoint_id = developer_webhook_endpoints.id
              AND delivery.status = 'failed'
          )::bigint AS failed_delivery_count
        "#,
    )
    .bind(endpoint_id)
    .bind(tenant_id)
    .bind(name)
    .bind(url)
    .bind(status)
    .fetch_optional(&mut *tx)
    .await
    .map_err(sql_status)?
    .ok_or_else(|| Status::not_found("developer webhook endpoint was not found"))?;

    sqlx::query("DELETE FROM developer_webhook_subscriptions WHERE endpoint_id = $1")
        .bind(endpoint_id)
        .execute(&mut *tx)
        .await
        .map_err(sql_status)?;
    insert_subscriptions(&mut tx, endpoint_id, &events).await?;
    tx.commit().await.map_err(sql_status)?;

    Ok(endpoint_with_events(endpoint, events, String::new()))
}

pub async fn delete_webhook_endpoint(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    endpoint_id: Uuid,
) -> Result<developer::WebhookEndpointDeletion, Status> {
    let row = sqlx::query(
        r#"
        UPDATE developer_webhook_endpoints
        SET status = 'revoked',
            revoked_at = NOW(),
            updated_at = NOW()
        WHERE id = $1
          AND tenant_id = $2
          AND revoked_at IS NULL
        RETURNING id, tenant_id
        "#,
    )
    .bind(endpoint_id)
    .bind(tenant_id)
    .fetch_optional(db)
    .await
    .map_err(sql_status)?
    .ok_or_else(|| Status::not_found("developer webhook endpoint was not found"))?;

    Ok(developer::WebhookEndpointDeletion {
        endpoint_id: row.get::<Uuid, _>("id").to_string(),
        tenant_id: row.get::<Uuid, _>("tenant_id").to_string(),
    })
}

pub async fn list_webhook_deliveries(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    endpoint_id: Uuid,
) -> Result<developer::ListWebhookDeliveriesResponse, Status> {
    let deliveries = sqlx::query(
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
    .map_err(sql_status)?
    .into_iter()
    .map(delivery_from_row)
    .collect();

    Ok(developer::ListWebhookDeliveriesResponse {
        deliveries,
        page: None,
    })
}

pub async fn replay_webhook_delivery(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    delivery_id: Uuid,
) -> Result<developer::WebhookDelivery, Status> {
    let original = sqlx::query(
        r#"
        SELECT d.id, d.endpoint_id, d.event_id, d.event_type, d.status::text AS status
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
    .await
    .map_err(sql_status)?
    .ok_or_else(|| Status::not_found("developer webhook delivery was not found"))?;

    ensure_replayable(original.get("status"))?;

    let replayed = sqlx::query(
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
    .bind(original.get::<Uuid, _>("endpoint_id"))
    .bind(tenant_id)
    .bind(original.get::<Uuid, _>("event_id"))
    .bind(original.get::<String, _>("event_type"))
    .bind(original.get::<Uuid, _>("id"))
    .fetch_one(db)
    .await
    .map_err(sql_status)?;

    Ok(delivery_from_row(replayed))
}

pub async fn list_api_logs(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
) -> Result<developer::ListApiLogsResponse, Status> {
    let logs = sqlx::query(
        r#"
        SELECT
          d.id,
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
    .fetch_all(db)
    .await
    .map_err(sql_status)?
    .into_iter()
    .map(api_log_from_row)
    .collect();

    Ok(developer::ListApiLogsResponse { logs, page: None })
}

#[cfg(test)]
#[path = "developer.grpc.webhooks.contract_tests.rs"]
mod contract_tests;
