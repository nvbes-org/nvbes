use chrono::{DateTime, Utc};
use sqlx::{Row, postgres::PgRow};
use tonic::Status;
use uuid::Uuid;

use crate::grpc::{
    pb::nvbes::developer::v1 as developer,
    service_status::{non_empty, parse_uuid, sql_status},
};

pub async fn list_marketplace_apps(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
) -> Result<developer::ListMarketplaceAppsResponse, Status> {
    let apps = sqlx::query(
        r#"
        SELECT client_id, status::text AS status, review_reason, created_at, updated_at
        FROM developer_marketplace_apps
        WHERE tenant_id = $1
        ORDER BY updated_at DESC
        "#,
    )
    .bind(tenant_id)
    .fetch_all(db)
    .await
    .map_err(sql_status)?
    .into_iter()
    .map(marketplace_app_from_row)
    .collect();

    Ok(developer::ListMarketplaceAppsResponse { apps })
}

pub async fn submit_marketplace_app(
    db: &sqlx::PgPool,
    request: developer::SubmitMarketplaceAppRequest,
) -> Result<developer::MarketplaceApp, Status> {
    let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
    let actor_id = actor_id(&request.context)?;
    let client_id = non_empty(request.client_id, "client_id")?;

    sqlx::query(
        r#"
        INSERT INTO developer_marketplace_apps (
          tenant_id,
          client_id,
          status,
          submitted_by,
          updated_at
        )
        VALUES ($1, $2, 'pending', $3, now())
        ON CONFLICT (tenant_id, client_id) DO UPDATE
        SET status = 'pending',
            submitted_by = EXCLUDED.submitted_by,
            review_reason = NULL,
            updated_at = now()
        "#,
    )
    .bind(tenant_id)
    .bind(&client_id)
    .bind(actor_id)
    .execute(db)
    .await
    .map_err(sql_status)?;

    get_marketplace_app(db, tenant_id, &client_id).await
}

pub async fn review_marketplace_app(
    db: &sqlx::PgPool,
    request: developer::ReviewMarketplaceAppRequest,
) -> Result<developer::MarketplaceApp, Status> {
    let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
    let actor_id = actor_id(&request.context)?;
    let client_id = non_empty(request.client_id, "client_id")?;
    let status = validate_marketplace_status(&request.status)?;
    let review_reason = empty_to_none(request.review_reason);

    let result = sqlx::query(
        r#"
        UPDATE developer_marketplace_apps
        SET status = $3::text::developer_marketplace_status,
            reviewed_by = $4,
            review_reason = $5,
            updated_at = now()
        WHERE tenant_id = $1
          AND client_id = $2
        "#,
    )
    .bind(tenant_id)
    .bind(&client_id)
    .bind(status)
    .bind(actor_id)
    .bind(review_reason)
    .execute(db)
    .await
    .map_err(sql_status)?;

    if result.rows_affected() == 0 {
        return Err(Status::failed_precondition(
            "Marketplace app has not been submitted yet",
        ));
    }

    get_marketplace_app(db, tenant_id, &client_id).await
}

async fn get_marketplace_app(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    client_id: &str,
) -> Result<developer::MarketplaceApp, Status> {
    sqlx::query(
        r#"
        SELECT client_id, status::text AS status, review_reason, created_at, updated_at
        FROM developer_marketplace_apps
        WHERE tenant_id = $1
          AND client_id = $2
        "#,
    )
    .bind(tenant_id)
    .bind(client_id)
    .fetch_optional(db)
    .await
    .map_err(sql_status)?
    .map(marketplace_app_from_row)
    .ok_or_else(|| Status::not_found("developer marketplace app was not found"))
}

fn marketplace_app_from_row(row: PgRow) -> developer::MarketplaceApp {
    developer::MarketplaceApp {
        client_id: row.get("client_id"),
        status: row.get("status"),
        review_reason: row
            .get::<Option<String>, _>("review_reason")
            .unwrap_or_default(),
        created_at: time_string(row.get("created_at")),
        updated_at: time_string(row.get("updated_at")),
    }
}

fn actor_id(
    context: &Option<crate::grpc::pb::nvbes::platform::v1::RequestContext>,
) -> Result<Uuid, Status> {
    let context = context
        .as_ref()
        .ok_or_else(|| Status::invalid_argument("request context is required"))?;
    parse_uuid(&context.actor_principal_id, "actor_principal_id")
}

fn validate_marketplace_status(value: &str) -> Result<&str, Status> {
    let status = value.trim();
    if matches!(status, "approved" | "rejected" | "suspended" | "pending") {
        Ok(status)
    } else {
        Err(Status::invalid_argument(
            "Marketplace status must be approved, rejected, suspended, or pending",
        ))
    }
}

fn empty_to_none(value: String) -> Option<String> {
    let value = value.trim().to_string();
    if value.is_empty() { None } else { Some(value) }
}

fn time_string(value: DateTime<Utc>) -> String {
    value.to_rfc3339()
}
