use std::collections::HashSet;

use sqlx::{PgPool, Postgres, Row, Transaction, postgres::PgRow};
use url::Url;
use uuid::Uuid;

use crate::{
    domains::developer::types::{
        CreateDeveloperWebhookEndpointRequest, CreateDeveloperWebhookEndpointResponse,
        DeveloperWebhookEndpointView, DeveloperWebhookEventType, DeveloperWebhooksResponse,
    },
    http::error::AppError,
};

pub async fn list_endpoints(
    db: &PgPool,
    tenant_id: Uuid,
) -> Result<DeveloperWebhooksResponse, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT
          endpoint.id,
          endpoint.name,
          endpoint.url,
          endpoint.status,
          endpoint.signing_secret_last4,
          endpoint.created_at,
          COALESCE(
            array_agg(subscription.event_type ORDER BY subscription.event_type)
              FILTER (WHERE subscription.event_type IS NOT NULL),
            ARRAY[]::text[]
          ) AS events
        FROM developer_webhook_endpoints endpoint
        LEFT JOIN developer_webhook_subscriptions subscription
          ON subscription.endpoint_id = endpoint.id
        WHERE endpoint.tenant_id = $1
          AND endpoint.revoked_at IS NULL
        GROUP BY endpoint.id
        ORDER BY endpoint.created_at DESC
        "#,
    )
    .bind(tenant_id)
    .fetch_all(db)
    .await?;

    Ok(DeveloperWebhooksResponse {
        endpoints: rows
            .into_iter()
            .map(|row| webhook_endpoint_view_from_row(&row))
            .collect::<Result<Vec<_>, _>>()?,
    })
}

pub async fn create_endpoint(
    db: &PgPool,
    tenant_id: Uuid,
    created_by: Uuid,
    request: CreateDeveloperWebhookEndpointRequest,
) -> Result<CreateDeveloperWebhookEndpointResponse, AppError> {
    validate_endpoint_request(&request)?;

    let signing_secret = format!("whsec_{}", Uuid::new_v4().simple());
    let signing_secret_last4 = signing_secret
        .chars()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>();
    let mut tx = db.begin().await?;
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
        RETURNING id, name, url, status, signing_secret_last4, created_at
        "#,
    )
    .bind(tenant_id)
    .bind(request.name.trim())
    .bind(request.url.trim())
    .bind(&signing_secret)
    .bind(&signing_secret_last4)
    .bind(created_by)
    .fetch_one(&mut *tx)
    .await?;

    insert_subscriptions(&mut tx, endpoint.get("id"), &request.events).await?;
    tx.commit().await?;

    Ok(CreateDeveloperWebhookEndpointResponse {
        endpoint: DeveloperWebhookEndpointView {
            id: endpoint.get("id"),
            name: endpoint.get("name"),
            url: endpoint.get("url"),
            status: endpoint.get("status"),
            events: sorted_unique_events(request.events),
            signing_secret_last4: endpoint.get("signing_secret_last4"),
            created_at: endpoint.get("created_at"),
        },
        signing_secret,
    })
}

pub async fn delete_endpoint(
    db: &PgPool,
    tenant_id: Uuid,
    endpoint_id: Uuid,
) -> Result<(), AppError> {
    let rows_affected = sqlx::query(
        r#"
        UPDATE developer_webhook_endpoints
        SET status = 'revoked',
            revoked_at = NOW(),
            updated_at = NOW()
        WHERE id = $1
          AND tenant_id = $2
          AND revoked_at IS NULL
        "#,
    )
    .bind(endpoint_id)
    .bind(tenant_id)
    .execute(db)
    .await?
    .rows_affected();

    if rows_affected == 0 {
        return Err(AppError::not_found(
            "developer_webhook_not_found",
            "Developer webhook endpoint not found.",
        ));
    }

    Ok(())
}

async fn insert_subscriptions(
    tx: &mut Transaction<'_, Postgres>,
    endpoint_id: Uuid,
    events: &[DeveloperWebhookEventType],
) -> Result<(), AppError> {
    for event in sorted_unique_events(events.to_vec()) {
        sqlx::query(
            r#"
            INSERT INTO developer_webhook_subscriptions (endpoint_id, event_type)
            VALUES ($1, $2)
            "#,
        )
        .bind(endpoint_id)
        .bind(event.as_event_type())
        .execute(&mut **tx)
        .await?;
    }

    Ok(())
}

fn validate_endpoint_request(
    request: &CreateDeveloperWebhookEndpointRequest,
) -> Result<(), AppError> {
    if request.name.trim().is_empty() {
        return Err(AppError::bad_request(
            "validation_failed",
            "Webhook endpoint name is required.",
        ));
    }
    if request.events.is_empty() {
        return Err(AppError::bad_request(
            "validation_failed",
            "At least one webhook event is required.",
        ));
    }

    let parsed_url = Url::parse(request.url.trim()).map_err(|_| {
        AppError::bad_request(
            "validation_failed",
            "Webhook endpoint URL must be absolute.",
        )
    })?;
    if !matches!(parsed_url.scheme(), "https" | "http") {
        return Err(AppError::bad_request(
            "validation_failed",
            "Webhook endpoint URL must use http or https.",
        ));
    }

    Ok(())
}

fn webhook_endpoint_view_from_row(row: &PgRow) -> Result<DeveloperWebhookEndpointView, AppError> {
    let event_types = row
        .get::<Vec<String>, _>("events")
        .into_iter()
        .map(|event| parse_webhook_event_type(&event))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(DeveloperWebhookEndpointView {
        id: row.get("id"),
        name: row.get("name"),
        url: row.get("url"),
        status: row.get("status"),
        events: event_types,
        signing_secret_last4: row.get("signing_secret_last4"),
        created_at: row.get("created_at"),
    })
}

fn parse_webhook_event_type(event_type: &str) -> Result<DeveloperWebhookEventType, AppError> {
    match event_type {
        "user.created" => Ok(DeveloperWebhookEventType::UserCreated),
        "login.failed" => Ok(DeveloperWebhookEventType::LoginFailed),
        "session.revoked" => Ok(DeveloperWebhookEventType::SessionRevoked),
        "client.created" => Ok(DeveloperWebhookEventType::ClientCreated),
        _ => Err(AppError::internal(
            "developer_webhook_event_unknown",
            format!("Unknown developer webhook event type: {event_type}."),
        )),
    }
}

fn sorted_unique_events(events: Vec<DeveloperWebhookEventType>) -> Vec<DeveloperWebhookEventType> {
    let mut seen = HashSet::new();
    let mut unique = Vec::new();

    for event in events {
        if seen.insert(event) {
            unique.push(event);
        }
    }
    unique.sort_by_key(|event| event.as_event_type());

    unique
}
