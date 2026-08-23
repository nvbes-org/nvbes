use sqlx::{Postgres, Transaction};
use tonic::Status;
use uuid::Uuid;

use crate::grpc::service_status::sql_status;

pub async fn insert_subscriptions(
    tx: &mut Transaction<'_, Postgres>,
    endpoint_id: Uuid,
    events: &[String],
) -> Result<(), Status> {
    for event in events {
        sqlx::query(
            r#"
            INSERT INTO developer_webhook_subscriptions (endpoint_id, event_type)
            VALUES ($1, $2)
            "#,
        )
        .bind(endpoint_id)
        .bind(event)
        .execute(&mut **tx)
        .await
        .map_err(sql_status)?;
    }
    Ok(())
}

pub async fn current_events(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    endpoint_id: Uuid,
) -> Result<Vec<String>, Status> {
    let exists = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
          SELECT 1
          FROM developer_webhook_endpoints
          WHERE id = $1 AND tenant_id = $2 AND revoked_at IS NULL
        )
        "#,
    )
    .bind(endpoint_id)
    .bind(tenant_id)
    .fetch_one(db)
    .await
    .map_err(sql_status)?;
    if !exists {
        return Err(Status::not_found(
            "developer webhook endpoint was not found",
        ));
    }

    sqlx::query_scalar(
        r#"
        SELECT event_type
        FROM developer_webhook_subscriptions
        WHERE endpoint_id = $1
        ORDER BY event_type
        "#,
    )
    .bind(endpoint_id)
    .fetch_all(db)
    .await
    .map_err(sql_status)
}
