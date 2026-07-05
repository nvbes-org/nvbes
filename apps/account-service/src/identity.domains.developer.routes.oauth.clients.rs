use axum::{Extension, Json, extract::State};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    app::AppState,
    domains::developer::{
        service,
        types::{DeveloperOAuthClientSummary, DeveloperOAuthClientsResponse},
    },
    http::{error::AppError, middleware::jwt::AuthContext},
};

pub async fn list_oauth_clients(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<DeveloperOAuthClientsResponse>, AppError> {
    let tenant_id = service::require_tenant_id(&auth)?;
    let oauth_clients = list_oauth_client_summaries(&state.db, tenant_id).await?;
    Ok(Json(DeveloperOAuthClientsResponse { oauth_clients }))
}

async fn list_oauth_client_summaries(
    db: &PgPool,
    tenant_id: Uuid,
) -> Result<Vec<DeveloperOAuthClientSummary>, AppError> {
    sqlx::query_as(
        r#"
        SELECT
          c.client_id,
          c.name,
          CASE WHEN c.revoked_at IS NULL THEN 'active' ELSE 'revoked' END AS status,
          m.status::text AS marketplace_status,
          cs.client_id IS NOT NULL AS consent_screen_configured,
          cardinality(c.redirect_uris)::bigint AS redirect_uri_count,
          COALESCE(policy.allowed_scopes, '{}'::text[]) AS allowed_scopes,
          COALESCE(health.status, 'unknown') AS health_status
        FROM oauth_clients c
        LEFT JOIN developer_marketplace_apps m
          ON m.tenant_id = c.tenant_id
         AND m.client_id = c.client_id
        LEFT JOIN developer_consent_screens cs
          ON cs.tenant_id = c.tenant_id
         AND cs.client_id = c.client_id
        LEFT JOIN LATERAL (
          SELECT array_agg(DISTINCT scope ORDER BY scope) AS allowed_scopes
          FROM oauth_client_policies p
          CROSS JOIN LATERAL unnest(p.allowed_scopes) AS scope
          WHERE p.client_id = c.id
            AND p.status = 'active'
        ) policy ON true
        LEFT JOIN LATERAL (
          SELECT h.status::text AS status
          FROM developer_health_checks h
          WHERE h.tenant_id = c.tenant_id
            AND h.target_type = 'oauth_client'
            AND h.target_id = c.client_id
          ORDER BY h.checked_at DESC
          LIMIT 1
        ) health ON true
        WHERE c.tenant_id = $1
        ORDER BY c.created_at DESC
        "#,
    )
    .bind(tenant_id)
    .fetch_all(db)
    .await
    .map_err(AppError::from)
}
