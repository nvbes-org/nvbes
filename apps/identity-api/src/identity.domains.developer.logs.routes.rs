use axum::{
    Extension, Json, Router,
    extract::{Query, State},
    routing::get,
};
use nvbes_core::http::error::ErrorEnvelope;
use serde::Deserialize;
use sqlx::Row;
use uuid::Uuid;

use crate::{
    app::AppState,
    domains::developer::{
        rbac::{self, DeveloperPermission},
        rbac_db,
        types::{DeveloperLogEntry, DeveloperLogsResponse},
    },
    http::{
        error::AppError,
        middleware::jwt::{AuthContext, jwt_auth_middleware},
    },
};

const DEVELOPER_LOG_LIMIT: i64 = 100;

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct DeveloperLogsQuery {
    pub user_id: Option<String>,
    pub client_id: Option<String>,
    pub tenant_id: Option<String>,
    pub event_type: Option<String>,
}

pub fn router(state: &AppState) -> Router<AppState> {
    Router::new().route(
        "/developer/logs",
        get(list_logs).layer(axum::middleware::from_fn_with_state(
            state.clone(),
            jwt_auth_middleware,
        )),
    )
}

#[utoipa::path(
    get,
    path = "/developer/logs",
    tag = "developer",
    params(
        ("user_id" = Option<String>, Query, description = "Principal/user ID"),
        ("client_id" = Option<String>, Query, description = "OAuth client ID"),
        ("tenant_id" = Option<String>, Query, description = "Tenant ID"),
        ("event_type" = Option<String>, Query, description = "Event type"),
    ),
    responses(
        (status = 200, description = "Developer logs", body = DeveloperLogsResponse),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 403, description = "Developer logs read permission required", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub async fn list_logs(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Query(query): Query<DeveloperLogsQuery>,
) -> Result<Json<DeveloperLogsResponse>, AppError> {
    let tenant_id =
        require_developer_permission(&state, &auth, DeveloperPermission::LogsRead).await?;
    let requested_tenant_id = parse_optional_uuid(query.tenant_id.as_deref(), "tenant_id")?;
    if requested_tenant_id.is_some_and(|requested| requested != tenant_id) {
        return Ok(Json(DeveloperLogsResponse { logs: Vec::new() }));
    }

    let user_id = parse_optional_uuid(query.user_id.as_deref(), "user_id")?;
    let client_id = query
        .client_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let event_type = query
        .event_type
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());

    let rows = sqlx::query(
        r#"
        SELECT id, event_type, user_id, client_id, tenant_id, created_at
        FROM (
          SELECT
            risk.id::text AS id,
            risk.event_type AS event_type,
            risk.principal_id AS user_id,
            risk.metadata->>'client_id' AS client_id,
            principal.tenant_id AS tenant_id,
            risk.created_at AS created_at
          FROM risk_events risk
          INNER JOIN principals principal ON principal.id = risk.principal_id
          WHERE principal.tenant_id = $1

          UNION ALL

          SELECT
            audit.id::text AS id,
            audit.action AS event_type,
            audit.actor_principal_id AS user_id,
            audit.metadata->>'client_id' AS client_id,
            audit.tenant_id AS tenant_id,
            audit.created_at AS created_at
          FROM audit_events audit
          WHERE audit.tenant_id = $1
        ) logs
        WHERE ($2::uuid IS NULL OR user_id = $2)
          AND ($3::text IS NULL OR client_id = $3)
          AND ($4::text IS NULL OR event_type = $4)
        ORDER BY created_at DESC
        LIMIT $5
        "#,
    )
    .bind(tenant_id)
    .bind(user_id)
    .bind(client_id)
    .bind(event_type)
    .bind(DEVELOPER_LOG_LIMIT)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(DeveloperLogsResponse {
        logs: rows
            .into_iter()
            .map(|row| DeveloperLogEntry {
                id: row.get("id"),
                event_type: row.get("event_type"),
                user_id: row
                    .get::<Option<Uuid>, _>("user_id")
                    .map(|value| value.to_string()),
                client_id: row.get("client_id"),
                tenant_id: row
                    .get::<Option<Uuid>, _>("tenant_id")
                    .map(|value| value.to_string()),
                created_at: row.get("created_at"),
            })
            .collect(),
    }))
}

async fn require_developer_permission(
    state: &AppState,
    auth: &AuthContext,
    required: DeveloperPermission,
) -> Result<Uuid, AppError> {
    let tenant_id = auth.tenant_id.ok_or_else(|| {
        AppError::forbidden(
            "tenant_context_required",
            "Tenant context is required for developer portal access.",
        )
    })?;
    let roles = rbac_db::load_developer_roles(&state.db, tenant_id, auth.user_id).await?;
    let permissions = rbac_db::permissions_for_roles(&roles);

    if !rbac::has_permission(&permissions, required) {
        return Err(AppError::forbidden(
            "developer_permission_required",
            format!("Developer permission {} is required.", required.as_scope()),
        ));
    }

    Ok(tenant_id)
}

fn parse_optional_uuid(value: Option<&str>, field: &str) -> Result<Option<Uuid>, AppError> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            Uuid::parse_str(value).map_err(|_| {
                AppError::bad_request("validation_failed", format!("{field} must be a UUID."))
            })
        })
        .transpose()
}
