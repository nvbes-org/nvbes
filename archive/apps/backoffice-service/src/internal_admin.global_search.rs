use axum::{
    Json, Router,
    extract::{Query, State},
    http::HeaderMap,
    routing::get,
};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::app::AppState;
use crate::billing_admin_access::actor_principal_id;
use crate::error::AppError;

#[derive(Debug, Deserialize)]
struct GlobalSearchQuery {
    q: String,
}

#[derive(Debug, Serialize)]
struct GlobalSearchResult {
    kind: String,
    id: Uuid,
    label: String,
    status: String,
    tenant_id: Option<Uuid>,
    workspace_id: Option<Uuid>,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/admin/search", get(global_search_route))
}

async fn global_search_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<GlobalSearchQuery>,
) -> Result<Json<Vec<GlobalSearchResult>>, AppError> {
    let _actor_id = actor_principal_id(&headers)?;
    let q = query.q.trim();
    if q.len() < 2 {
        return Err(AppError::bad_request(
            "invalid_global_search_query",
            "Global back-office search requires at least two characters.",
        ));
    }
    Ok(Json(global_search(&state.db, q).await?))
}

async fn global_search(db: &PgPool, query: &str) -> Result<Vec<GlobalSearchResult>, AppError> {
    let pattern = format!("%{query}%");
    let rows = sqlx::query(
        r#"
        SELECT kind, id, label, status, tenant_id, workspace_id FROM (
          SELECT 'tenant' AS kind, t.id, t.name || ' / ' || t.slug AS label, t.status::text AS status,
            t.id AS tenant_id, NULL::uuid AS workspace_id
          FROM tenants t
          WHERE t.name ILIKE $1 OR t.slug ILIKE $1 OR t.id::text ILIKE $1

          UNION ALL

          SELECT 'workspace' AS kind, w.id, w.name || ' / ' || w.plan_code AS label,
            w.status::text AS status, w.tenant_id, w.id AS workspace_id
          FROM workspaces w
          WHERE w.name ILIKE $1 OR w.id::text ILIKE $1

          UNION ALL

          SELECT 'user' AS kind, u.principal_id AS id, u.email || ' / ' || u.name AS label,
            u.status::text AS status, p.tenant_id, NULL::uuid AS workspace_id
          FROM users u
          JOIN principals p ON p.id = u.principal_id
          WHERE u.email ILIKE $1 OR u.name ILIKE $1 OR u.principal_id::text ILIKE $1
        ) results
        ORDER BY kind, label
        LIMIT 30
        "#,
    )
    .bind(pattern)
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| GlobalSearchResult {
            kind: row.get("kind"),
            id: row.get("id"),
            label: row.get("label"),
            status: row.get("status"),
            tenant_id: row.get("tenant_id"),
            workspace_id: row.get("workspace_id"),
        })
        .collect())
}
