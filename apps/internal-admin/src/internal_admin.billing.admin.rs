use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderName, header},
    response::IntoResponse,
    routing::{get, post},
};
use uuid::Uuid;

use crate::app::AppState;
use crate::backoffice_authorization::{
    BackofficePermission, require_confirmation, require_idempotency_key, require_permission,
};
use crate::billing_admin_access::authorize_backoffice;
use crate::billing_admin_exports::{build_finance_export, parse_finance_export_type};
use crate::billing_admin_mutations::{
    create_credit_note, create_manual_compensation, create_provider_migration,
    create_refund_intent, create_write_off, override_grace_period, replay_provider_event,
};
use crate::billing_admin_search::search_billing_admin;
use crate::billing_admin_types::{
    CreditNoteRequest, GraceOverrideRequest, ManualCompRequest, MutationResult,
    ProviderMigrationRequest, ProviderReplayRequest, ProviderReplayResult, RefundIntentRequest,
    SearchQuery, SearchResult,
};
use crate::error::AppError;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/workspaces/{workspaceId}/billing/admin/search",
            get(search_route),
        )
        .route(
            "/workspaces/{workspaceId}/billing/admin/credit-notes",
            post(credit_note_route),
        )
        .route(
            "/workspaces/{workspaceId}/billing/admin/write-offs",
            post(write_off_route),
        )
        .route(
            "/workspaces/{workspaceId}/billing/admin/refund-intents",
            post(refund_intent_route),
        )
        .route(
            "/workspaces/{workspaceId}/billing/admin/provider-events/replay",
            post(replay_provider_event_route),
        )
        .route(
            "/workspaces/{workspaceId}/billing/admin/provider-migrations",
            post(provider_migration_route),
        )
        .route(
            "/workspaces/{workspaceId}/billing/admin/grace-overrides",
            post(grace_override_route),
        )
        .route(
            "/workspaces/{workspaceId}/billing/admin/manual-compensations",
            post(manual_comp_route),
        )
        .route(
            "/workspaces/{workspaceId}/billing/admin/exports/{exportType}",
            post(finance_export_route),
        )
}

async fn search_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<Vec<SearchResult>>, AppError> {
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        search_billing_admin(&state.db, access.tenant_id, &query.q).await?,
    ))
}

async fn credit_note_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<CreditNoteRequest>,
) -> Result<Json<MutationResult>, AppError> {
    require_idempotency_key(&headers)?;
    require_permission(&headers, BackofficePermission::BillingMutate)?;
    require_confirmation(&request.confirm_code, "CREATE CREDIT NOTE")?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(create_credit_note(&state.db, access, request).await?))
}

async fn write_off_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<CreditNoteRequest>,
) -> Result<Json<MutationResult>, AppError> {
    require_idempotency_key(&headers)?;
    require_permission(&headers, BackofficePermission::BillingMutate)?;
    require_confirmation(&request.confirm_code, "WRITE OFF")?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(create_write_off(&state.db, access, request).await?))
}

async fn refund_intent_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<RefundIntentRequest>,
) -> Result<Json<MutationResult>, AppError> {
    require_idempotency_key(&headers)?;
    require_permission(&headers, BackofficePermission::BillingMutate)?;
    require_confirmation(&request.confirm_code, "CREATE REFUND")?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        create_refund_intent(&state.db, access, request).await?,
    ))
}

async fn replay_provider_event_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<ProviderReplayRequest>,
) -> Result<Json<ProviderReplayResult>, AppError> {
    require_idempotency_key(&headers)?;
    require_permission(&headers, BackofficePermission::BillingMutate)?;
    require_confirmation(&request.confirm_code, "REPLAY EVENT")?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        replay_provider_event(&state.db, access, request).await?,
    ))
}

async fn provider_migration_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<ProviderMigrationRequest>,
) -> Result<Json<MutationResult>, AppError> {
    require_idempotency_key(&headers)?;
    require_permission(&headers, BackofficePermission::BillingMutate)?;
    require_confirmation(&request.confirm_code, "PLAN MIGRATION")?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        create_provider_migration(&state.db, access, request).await?,
    ))
}

async fn grace_override_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<GraceOverrideRequest>,
) -> Result<Json<MutationResult>, AppError> {
    require_idempotency_key(&headers)?;
    require_permission(&headers, BackofficePermission::BillingMutate)?;
    require_confirmation(&request.confirm_code, "OVERRIDE GRACE")?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        override_grace_period(&state.db, access, workspace_id, request).await?,
    ))
}

async fn manual_comp_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<ManualCompRequest>,
) -> Result<Json<MutationResult>, AppError> {
    require_idempotency_key(&headers)?;
    require_permission(&headers, BackofficePermission::BillingMutate)?;
    require_confirmation(&request.confirm_code, "CREATE COMPENSATION")?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        create_manual_compensation(&state.db, access, request).await?,
    ))
}

async fn finance_export_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, export_type)): Path<(Uuid, String)>,
) -> Result<impl IntoResponse, AppError> {
    require_idempotency_key(&headers)?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    let export_type = parse_finance_export_type(&export_type)?;
    let export = build_finance_export(&state.db, access.tenant_id, export_type).await?;
    Ok((
        [
            (header::CONTENT_TYPE, export.content_type.to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}\"", export.filename),
            ),
            (header::CACHE_CONTROL, "no-store".to_string()),
            (
                HeaderName::from_static("x-nvbes-billing-export-run-id"),
                export.export_run_id.to_string(),
            ),
            (
                HeaderName::from_static("x-nvbes-billing-export-row-count"),
                export.row_count.to_string(),
            ),
        ],
        export.body,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Router,
        body::{self, Body},
        http::{Request, StatusCode},
    };
    use serde_json::json;
    use sqlx::{PgPool, Row, postgres::PgPoolOptions};
    use std::time::Duration;
    use tower::ServiceExt;

    #[tokio::test]
    async fn replay_provider_event_route_enforces_role_confirmation_and_audits_success() {
        let Some(pool) = test_pool().await else {
            eprintln!("skipping test: Postgres is not reachable");
            return;
        };
        if !billing_replay_schema_exists(&pool).await {
            eprintln!("skipping test: billing replay schema is missing");
            return;
        }

        let actor_id = Uuid::new_v4();
        let provider_event_id = format!("evt_{}", Uuid::new_v4());
        let (tenant_id, workspace_id, event_id) =
            seed_workspace_actor_and_provider_event(&pool, actor_id, &provider_event_id).await;
        let app = Router::new()
            .merge(router())
            .with_state(crate::app::AppState::new(
                nvbes_core::config::AppConfig::default(),
                pool.clone(),
            ));

        let denied = app
            .clone()
            .oneshot(replay_request(
                workspace_id,
                actor_id,
                "viewer",
                "REPLAY EVENT",
                &provider_event_id,
                "ticket BILL-456 approved",
            ))
            .await
            .expect("route should respond");
        assert_eq!(denied.status(), StatusCode::FORBIDDEN);

        let wrong_confirmation = app
            .clone()
            .oneshot(replay_request(
                workspace_id,
                actor_id,
                "finance_admin",
                "REPLAY",
                &provider_event_id,
                "ticket BILL-456 approved",
            ))
            .await
            .expect("route should respond");
        assert_eq!(wrong_confirmation.status(), StatusCode::BAD_REQUEST);

        let accepted = app
            .oneshot(replay_request(
                workspace_id,
                actor_id,
                "finance_admin",
                "REPLAY EVENT",
                &provider_event_id,
                "ticket BILL-456 approved",
            ))
            .await
            .expect("route should respond");
        assert_eq!(accepted.status(), StatusCode::OK);
        let body = body::to_bytes(accepted.into_body(), usize::MAX)
            .await
            .expect("body should be readable");
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("body should be json");
        assert_eq!(payload["object_id"], json!(event_id.to_string()));
        assert_eq!(payload["status"], json!("replayed"));

        let status = sqlx::query_scalar::<_, String>(
            "SELECT status::text FROM billing_provider_events WHERE id = $1",
        )
        .bind(event_id)
        .fetch_one(&pool)
        .await
        .expect("provider event should exist");
        assert_eq!(status, "replayed");

        let audit_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM audit_events
             WHERE tenant_id = $1 AND actor_principal_id = $2
               AND action = 'billing.provider_event.replayed'
               AND target_type = 'billing_provider_event'
               AND target_id = $3",
        )
        .bind(tenant_id)
        .bind(actor_id)
        .bind(event_id)
        .fetch_one(&pool)
        .await
        .expect("audit count should load");
        assert_eq!(audit_count, 1);
    }

    async fn test_pool() -> Option<PgPool> {
        let database_url = std::env::var("DATABASE_URL")
            .or_else(|_| std::env::var("NVBES_DATABASE_URL"))
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/nvbes".to_string());
        tokio::time::timeout(
            Duration::from_secs(2),
            PgPoolOptions::new()
                .max_connections(1)
                .connect(&database_url),
        )
        .await
        .ok()
        .and_then(Result::ok)
    }

    async fn billing_replay_schema_exists(pool: &PgPool) -> bool {
        sqlx::query_scalar::<_, bool>(
            "SELECT to_regclass('public.tenants') IS NOT NULL
              AND to_regclass('public.principals') IS NOT NULL
              AND to_regclass('public.workspaces') IS NOT NULL
              AND to_regclass('public.billing_provider_events') IS NOT NULL
              AND to_regclass('public.audit_events') IS NOT NULL",
        )
        .fetch_one(pool)
        .await
        .unwrap_or(false)
    }

    async fn seed_workspace_actor_and_provider_event(
        pool: &PgPool,
        actor_id: Uuid,
        provider_event_id: &str,
    ) -> (Uuid, Uuid, Uuid) {
        let tenant_id = Uuid::new_v4();
        let workspace_id = Uuid::new_v4();
        let event_id = Uuid::new_v4();
        let slug = format!("test-billing-tenant-{}", Uuid::new_v4());
        sqlx::query(
            "INSERT INTO tenants (id, kind, name, slug, status, security_tier)
             VALUES ($1, 'enterprise', 'Test Billing Tenant', $2, 'active', 'standard')",
        )
        .bind(tenant_id)
        .bind(slug)
        .execute(pool)
        .await
        .expect("tenant should insert");

        sqlx::query(
            "INSERT INTO principals (id, tenant_id, principal_kind, status, display_name)
             VALUES ($1, $2, 'human', 'active', 'Backoffice Actor')",
        )
        .bind(actor_id)
        .bind(tenant_id)
        .execute(pool)
        .await
        .expect("actor should insert");

        insert_workspace(pool, workspace_id, tenant_id).await;

        sqlx::query(
            "INSERT INTO billing_provider_events (
               id, tenant_id, provider, provider_event_id, event_type, status,
               signature_valid, payload_hash, payload_summary
             ) VALUES (
               $1, $2, 'stripe', $3, 'invoice.payment_failed', 'failed',
               true, 'hash-test', '{}'::jsonb
             )",
        )
        .bind(event_id)
        .bind(tenant_id)
        .bind(provider_event_id)
        .execute(pool)
        .await
        .expect("provider event should insert");

        (tenant_id, workspace_id, event_id)
    }

    async fn insert_workspace(pool: &PgPool, workspace_id: Uuid, tenant_id: Uuid) {
        if workspace_status_column_exists(pool).await {
            sqlx::query(
                "INSERT INTO workspaces (id, tenant_id, name, workspace_type, plan_code, status)
                 VALUES ($1, $2, 'Billing Workspace', 'team', 'team', 'active')",
            )
            .bind(workspace_id)
            .bind(tenant_id)
            .execute(pool)
            .await
            .expect("workspace should insert");
            return;
        }
        sqlx::query(
            "INSERT INTO workspaces (id, tenant_id, name, workspace_type, plan_code)
             VALUES ($1, $2, 'Billing Workspace', 'team', 'team')",
        )
        .bind(workspace_id)
        .bind(tenant_id)
        .execute(pool)
        .await
        .expect("workspace should insert");
    }

    async fn workspace_status_column_exists(pool: &PgPool) -> bool {
        sqlx::query(
            "SELECT 1 FROM information_schema.columns
             WHERE table_schema = 'public' AND table_name = 'workspaces' AND column_name = 'status'",
        )
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .map(|row| row.get::<i32, _>(0) == 1)
        .unwrap_or(false)
    }

    fn replay_request(
        workspace_id: Uuid,
        actor_id: Uuid,
        role: &str,
        confirm_code: &str,
        provider_event_id: &str,
        reason: &str,
    ) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri(format!(
                "/workspaces/{workspace_id}/billing/admin/provider-events/replay"
            ))
            .header("content-type", "application/json")
            .header("idempotency-key", format!("test-{}", Uuid::new_v4()))
            .header("x-nvbes-actor-principal-id", actor_id.to_string())
            .header("x-nvbes-backoffice-role", role)
            .body(Body::from(
                json!({
                    "confirm_code": confirm_code,
                    "provider": "stripe",
                    "provider_event_id": provider_event_id,
                    "reason": reason
                })
                .to_string(),
            ))
            .expect("request should build")
    }
}
