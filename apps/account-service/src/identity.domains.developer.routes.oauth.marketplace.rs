use axum::{
    Extension, Json,
    extract::{Path, State},
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    app::AppState,
    domains::developer::{
        rbac::DeveloperPermission,
        service,
        types::{
            DeveloperMarketplaceAppSummary, DeveloperMarketplaceAppsResponse,
            ReviewMarketplaceAppInput,
        },
    },
    http::{error::AppError, middleware::jwt::AuthContext},
};

use super::consent::ensure_oauth_client_in_tenant;

pub async fn list_marketplace_apps(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<DeveloperMarketplaceAppsResponse>, AppError> {
    let tenant_id = service::require_tenant_id(&auth)?;
    let apps = list_marketplace_app_summaries(&state.db, tenant_id).await?;
    Ok(Json(DeveloperMarketplaceAppsResponse { apps }))
}

pub async fn submit_marketplace_app(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(client_id): Path<String>,
) -> Result<Json<DeveloperMarketplaceAppSummary>, AppError> {
    let tenant_id = service::require_permission(
        &state.db,
        &auth,
        DeveloperPermission::ConsoleMarketplaceSubmit,
    )
    .await?;

    ensure_oauth_client_in_tenant(&state.db, tenant_id, &client_id).await?;

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
    .bind(auth.user_id)
    .execute(&state.db)
    .await?;

    let app = get_marketplace_app_summary(&state.db, tenant_id, &client_id).await?;
    Ok(Json(app))
}

pub async fn review_marketplace_app(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(client_id): Path<String>,
    Json(input): Json<ReviewMarketplaceAppInput>,
) -> Result<Json<DeveloperMarketplaceAppSummary>, AppError> {
    let tenant_id =
        service::require_permission(&state.db, &auth, DeveloperPermission::MarketplaceReview)
            .await?;

    ensure_oauth_client_in_tenant(&state.db, tenant_id, &client_id).await?;

    if !matches!(
        input.status.as_str(),
        "approved" | "rejected" | "suspended" | "pending"
    ) {
        return Err(AppError::bad_request(
            "invalid_marketplace_status",
            "Marketplace status must be 'approved', 'rejected', 'suspended', or 'pending'",
        ));
    }

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
    .bind(input.status)
    .bind(auth.user_id)
    .bind(input.review_reason)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::bad_request(
            "marketplace_app_not_submitted",
            "Marketplace app has not been submitted yet",
        ));
    }

    let app = get_marketplace_app_summary(&state.db, tenant_id, &client_id).await?;
    Ok(Json(app))
}

async fn list_marketplace_app_summaries(
    db: &PgPool,
    tenant_id: Uuid,
) -> Result<Vec<DeveloperMarketplaceAppSummary>, AppError> {
    sqlx::query_as(
        r#"
        SELECT
          m.client_id,
          c.name,
          m.status::text AS status,
          m.review_reason,
          m.created_at,
          m.updated_at
        FROM developer_marketplace_apps m
        INNER JOIN oauth_clients c
          ON c.tenant_id = m.tenant_id
         AND c.client_id = m.client_id
        WHERE m.tenant_id = $1
        ORDER BY m.updated_at DESC
        "#,
    )
    .bind(tenant_id)
    .fetch_all(db)
    .await
    .map_err(AppError::from)
}

async fn get_marketplace_app_summary(
    db: &PgPool,
    tenant_id: Uuid,
    client_id: &str,
) -> Result<DeveloperMarketplaceAppSummary, AppError> {
    sqlx::query_as(
        r#"
        SELECT
          m.client_id,
          c.name,
          m.status::text AS status,
          m.review_reason,
          m.created_at,
          m.updated_at
        FROM developer_marketplace_apps m
        INNER JOIN oauth_clients c
          ON c.tenant_id = m.tenant_id
         AND c.client_id = m.client_id
        WHERE m.tenant_id = $1
          AND m.client_id = $2
        "#,
    )
    .bind(tenant_id)
    .bind(client_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::not_found("marketplace_app_not_found", "Marketplace app not found"))
}
