use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, post},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::app::AppState;
use crate::backoffice_authorization::{
    BackofficePermission, require_confirmation, require_idempotency_key, require_permission,
};
use crate::billing_admin_access::actor_principal_id;
use crate::error::AppError;

#[derive(Debug, Serialize)]
struct UserDetail {
    principal_id: Uuid,
    tenant_id: Uuid,
    tenant_name: String,
    email: String,
    name: String,
    principal_status: String,
    user_status: String,
    email_verified_at: Option<DateTime<Utc>>,
    workspace_count: i64,
    active_workspace_count: i64,
    active_mfa_factor_count: i64,
    active_oauth_consent_count: i64,
    risk_events_24h: i64,
    audit_events_24h: i64,
    latest_risk_at: Option<DateTime<Utc>>,
    latest_audit_at: Option<DateTime<Utc>>,
    primary_workspace_id: Option<Uuid>,
    primary_workspace_name: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct UserLifecycleRequest {
    confirm_code: String,
    reason: String,
}

#[derive(Debug, Serialize)]
struct UserLifecycleResult {
    principal_id: Uuid,
    tenant_id: Uuid,
    previous_principal_status: String,
    previous_user_status: String,
    next_principal_status: String,
    next_user_status: String,
    audit_action: &'static str,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/admin/users/{principalId}", get(user_detail_route))
        .route(
            "/admin/users/{principalId}/suspend",
            post(suspend_user_route),
        )
        .route(
            "/admin/users/{principalId}/reactivate",
            post(reactivate_user_route),
        )
}

async fn user_detail_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(principal_id): Path<Uuid>,
) -> Result<Json<UserDetail>, AppError> {
    let _actor_id = actor_principal_id(&headers)?;
    Ok(Json(load_user_detail(&state.db, principal_id).await?))
}

async fn suspend_user_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(principal_id): Path<Uuid>,
    Json(request): Json<UserLifecycleRequest>,
) -> Result<Json<UserLifecycleResult>, AppError> {
    require_idempotency_key(&headers)?;
    require_permission(&headers, BackofficePermission::UserLifecycle)?;
    require_confirmation(&request.confirm_code, "SUSPEND USER")?;
    let actor_id = actor_principal_id(&headers)?;
    Ok(Json(
        change_user_status(
            &state.db,
            actor_id,
            principal_id,
            UserLifecycleTarget {
                principal_status: "suspended",
                user_status: "suspended",
                audit_action: "internal_admin.user.suspend",
            },
            request,
        )
        .await?,
    ))
}

async fn reactivate_user_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(principal_id): Path<Uuid>,
    Json(request): Json<UserLifecycleRequest>,
) -> Result<Json<UserLifecycleResult>, AppError> {
    require_idempotency_key(&headers)?;
    require_permission(&headers, BackofficePermission::UserLifecycle)?;
    require_confirmation(&request.confirm_code, "REACTIVATE USER")?;
    let actor_id = actor_principal_id(&headers)?;
    Ok(Json(
        change_user_status(
            &state.db,
            actor_id,
            principal_id,
            UserLifecycleTarget {
                principal_status: "active",
                user_status: "active",
                audit_action: "internal_admin.user.reactivate",
            },
            request,
        )
        .await?,
    ))
}

async fn load_user_detail(db: &PgPool, principal_id: Uuid) -> Result<UserDetail, AppError> {
    let row = sqlx::query(
        r#"
        SELECT
          p.id, p.tenant_id, t.name AS tenant_name, u.email, u.name,
          p.status::text AS principal_status, u.status::text AS user_status,
          u.email_verified_at, u.created_at, u.updated_at,
          (
            SELECT COUNT(*) FROM workspace_memberships wm
            WHERE wm.principal_id = p.id
          ) AS workspace_count,
          (
            SELECT COUNT(*) FROM workspace_memberships wm
            WHERE wm.principal_id = p.id AND wm.status::text = 'active'
          ) AS active_workspace_count,
          (
            SELECT COUNT(*) FROM mfa_factors mf
            WHERE mf.principal_id = p.id AND mf.status::text = 'active'
          ) AS active_mfa_factor_count,
          (
            SELECT COUNT(*) FROM oauth_consents oc
            WHERE oc.principal_id = p.id AND oc.revoked_at IS NULL
              AND (oc.expires_at IS NULL OR oc.expires_at > NOW())
          ) AS active_oauth_consent_count,
          (
            SELECT COUNT(*) FROM risk_events re
            WHERE re.principal_id = p.id AND re.created_at >= NOW() - INTERVAL '24 hours'
          ) AS risk_events_24h,
          (
            SELECT COUNT(*) FROM audit_events ae
            WHERE ae.actor_principal_id = p.id AND ae.created_at >= NOW() - INTERVAL '24 hours'
          ) AS audit_events_24h,
          (SELECT MAX(re.created_at) FROM risk_events re WHERE re.principal_id = p.id)
            AS latest_risk_at,
          (SELECT MAX(ae.created_at) FROM audit_events ae WHERE ae.actor_principal_id = p.id)
            AS latest_audit_at,
          (
            SELECT wm.workspace_id FROM workspace_memberships wm
            WHERE wm.principal_id = p.id
            ORDER BY wm.updated_at DESC
            LIMIT 1
          ) AS primary_workspace_id,
          (
            SELECT w.name FROM workspace_memberships wm
            JOIN workspaces w ON w.id = wm.workspace_id
            WHERE wm.principal_id = p.id
            ORDER BY wm.updated_at DESC
            LIMIT 1
          ) AS primary_workspace_name
        FROM principals p
        JOIN tenants t ON t.id = p.tenant_id
        JOIN users u ON u.principal_id = p.id
        WHERE p.id = $1
        "#,
    )
    .bind(principal_id)
    .fetch_one(db)
    .await?;

    Ok(UserDetail {
        principal_id: row.get(0),
        tenant_id: row.get(1),
        tenant_name: row.get(2),
        email: row.get(3),
        name: row.get(4),
        principal_status: row.get(5),
        user_status: row.get(6),
        email_verified_at: row.get(7),
        created_at: row.get(8),
        updated_at: row.get(9),
        workspace_count: row.get(10),
        active_workspace_count: row.get(11),
        active_mfa_factor_count: row.get(12),
        active_oauth_consent_count: row.get(13),
        risk_events_24h: row.get(14),
        audit_events_24h: row.get(15),
        latest_risk_at: row.get(16),
        latest_audit_at: row.get(17),
        primary_workspace_id: row.get(18),
        primary_workspace_name: row.get(19),
    })
}

#[derive(Debug, Clone, Copy)]
struct UserLifecycleTarget {
    principal_status: &'static str,
    user_status: &'static str,
    audit_action: &'static str,
}

async fn change_user_status(
    db: &PgPool,
    actor_id: Uuid,
    principal_id: Uuid,
    target: UserLifecycleTarget,
    request: UserLifecycleRequest,
) -> Result<UserLifecycleResult, AppError> {
    validate_lifecycle_reason(&request.reason)?;

    let mut tx = db.begin().await?;
    let row = sqlx::query(
        r#"
        SELECT p.tenant_id, p.status::text AS principal_status, u.status::text AS user_status
        FROM principals p
        JOIN users u ON u.principal_id = p.id
        WHERE p.id = $1
        FOR UPDATE OF p, u
        "#,
    )
    .bind(principal_id)
    .fetch_one(tx.as_mut())
    .await?;
    let tenant_id: Uuid = row.get("tenant_id");
    let previous_principal_status: String = row.get("principal_status");
    let previous_user_status: String = row.get("user_status");

    validate_user_status_transition(
        &previous_principal_status,
        &previous_user_status,
        target.principal_status,
        target.user_status,
    )?;

    sqlx::query(
        "UPDATE principals SET status = $2::principal_status, updated_at = NOW() WHERE id = $1",
    )
    .bind(principal_id)
    .bind(target.principal_status)
    .execute(tx.as_mut())
    .await?;

    sqlx::query(
        "UPDATE users SET status = $2::user_status, updated_at = NOW() WHERE principal_id = $1",
    )
    .bind(principal_id)
    .bind(target.user_status)
    .execute(tx.as_mut())
    .await?;

    sqlx::query(
        "INSERT INTO audit_events (
           tenant_id, actor_principal_id, action, target_type, target_id, metadata, event_hash
         ) VALUES (
           $1, $2, $3, 'principal', $4,
           jsonb_build_object(
             'reason', $5,
             'previous_principal_status', $6,
             'previous_user_status', $7,
             'next_principal_status', $8,
             'next_user_status', $9
           ),
           gen_random_uuid()::text
         )",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .bind(target.audit_action)
    .bind(principal_id)
    .bind(request.reason.trim())
    .bind(&previous_principal_status)
    .bind(&previous_user_status)
    .bind(target.principal_status)
    .bind(target.user_status)
    .execute(tx.as_mut())
    .await?;

    tx.commit().await?;

    Ok(UserLifecycleResult {
        principal_id,
        tenant_id,
        previous_principal_status,
        previous_user_status,
        next_principal_status: target.principal_status.to_string(),
        next_user_status: target.user_status.to_string(),
        audit_action: target.audit_action,
    })
}

fn validate_lifecycle_reason(reason: &str) -> Result<(), AppError> {
    if reason.trim().len() < 12 {
        return Err(AppError::bad_request(
            "audit_reason_required",
            "User lifecycle actions require a detailed audit reason.",
        ));
    }
    Ok(())
}

fn validate_user_status_transition(
    previous_principal_status: &str,
    previous_user_status: &str,
    next_principal_status: &'static str,
    next_user_status: &'static str,
) -> Result<(), AppError> {
    if matches!(previous_principal_status, "deleted" | "revoked")
        || previous_user_status == "deleted"
    {
        return Err(AppError::bad_request(
            "user_lifecycle_terminal",
            "Deleted or revoked users cannot be mutated from the back-office.",
        ));
    }
    if previous_principal_status == next_principal_status
        && previous_user_status == next_user_status
    {
        return Err(AppError::bad_request(
            "user_status_unchanged",
            "User is already in the requested status.",
        ));
    }
    Ok(())
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
    use sqlx::postgres::PgPoolOptions;
    use std::time::Duration;
    use tower::ServiceExt;

    #[test]
    fn lifecycle_reason_must_be_detailed() {
        assert!(validate_lifecycle_reason("too short").is_err());
        assert!(validate_lifecycle_reason("incident SEC-123 approved").is_ok());
    }

    #[test]
    fn user_lifecycle_rejects_terminal_and_unchanged_statuses() {
        assert!(validate_user_status_transition("deleted", "active", "active", "active").is_err());
        assert!(validate_user_status_transition("revoked", "active", "active", "active").is_err());
        assert!(validate_user_status_transition("active", "deleted", "active", "active").is_err());
        assert!(
            validate_user_status_transition("suspended", "suspended", "suspended", "suspended")
                .is_err()
        );
        assert!(
            validate_user_status_transition("active", "active", "suspended", "suspended").is_ok()
        );
    }

    #[tokio::test]
    async fn suspend_user_route_enforces_role_confirmation_and_audits_success() {
        let Some(pool) = test_pool().await else {
            eprintln!("skipping test: Postgres is not reachable");
            return;
        };
        if !user_lifecycle_schema_exists(&pool).await {
            eprintln!("skipping test: user lifecycle schema is missing");
            return;
        }

        let actor_id = Uuid::new_v4();
        let target_id = Uuid::new_v4();
        let tenant_id = seed_tenant_actor_and_user(&pool, actor_id, target_id).await;
        let app = Router::new()
            .merge(router())
            .with_state(crate::app::AppState::new(
                nvbes_core::config::AppConfig::default(),
                pool.clone(),
            ));

        let denied = app
            .clone()
            .oneshot(suspend_request(
                target_id,
                actor_id,
                "finance_admin",
                "SUSPEND USER",
                "ticket SEC-456 approved",
            ))
            .await
            .expect("route should respond");
        assert_eq!(denied.status(), StatusCode::FORBIDDEN);

        let wrong_confirmation = app
            .clone()
            .oneshot(suspend_request(
                target_id,
                actor_id,
                "security_admin",
                "SUSPEND",
                "ticket SEC-456 approved",
            ))
            .await
            .expect("route should respond");
        assert_eq!(wrong_confirmation.status(), StatusCode::BAD_REQUEST);

        let accepted = app
            .oneshot(suspend_request(
                target_id,
                actor_id,
                "security_admin",
                "SUSPEND USER",
                "ticket SEC-456 approved",
            ))
            .await
            .expect("route should respond");
        assert_eq!(accepted.status(), StatusCode::OK);
        let body = body::to_bytes(accepted.into_body(), usize::MAX)
            .await
            .expect("body should be readable");
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("body should be json");
        assert_eq!(payload["next_principal_status"], json!("suspended"));
        assert_eq!(payload["next_user_status"], json!("suspended"));

        let row = sqlx::query(
            "SELECT p.status::text AS principal_status, u.status::text AS user_status
             FROM principals p JOIN users u ON u.principal_id = p.id
             WHERE p.id = $1",
        )
        .bind(target_id)
        .fetch_one(&pool)
        .await
        .expect("target user should exist");
        let principal_status: String = row.get("principal_status");
        let user_status: String = row.get("user_status");
        assert_eq!(principal_status, "suspended");
        assert_eq!(user_status, "suspended");

        let audit_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM audit_events
             WHERE tenant_id = $1 AND actor_principal_id = $2
               AND action = 'internal_admin.user.suspend'
               AND target_type = 'principal'
               AND target_id = $3",
        )
        .bind(tenant_id)
        .bind(actor_id)
        .bind(target_id)
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

    async fn user_lifecycle_schema_exists(pool: &PgPool) -> bool {
        sqlx::query_scalar::<_, bool>(
            "SELECT to_regclass('public.tenants') IS NOT NULL
              AND to_regclass('public.principals') IS NOT NULL
              AND to_regclass('public.users') IS NOT NULL
              AND to_regclass('public.audit_events') IS NOT NULL",
        )
        .fetch_one(pool)
        .await
        .unwrap_or(false)
    }

    async fn seed_tenant_actor_and_user(pool: &PgPool, actor_id: Uuid, target_id: Uuid) -> Uuid {
        let tenant_id = Uuid::new_v4();
        let slug = format!("test-user-tenant-{}", Uuid::new_v4());
        let email = format!("{}@example.test", Uuid::new_v4());
        sqlx::query(
            "INSERT INTO tenants (id, kind, name, slug, status, security_tier)
             VALUES ($1, 'enterprise', 'Test User Tenant', $2, 'active', 'standard')",
        )
        .bind(tenant_id)
        .bind(slug)
        .execute(pool)
        .await
        .expect("tenant should insert");

        for (principal_id, display_name) in
            [(actor_id, "Backoffice Actor"), (target_id, "Target User")]
        {
            sqlx::query(
                "INSERT INTO principals (id, tenant_id, principal_kind, status, display_name)
                 VALUES ($1, $2, 'human', 'active', $3)",
            )
            .bind(principal_id)
            .bind(tenant_id)
            .bind(display_name)
            .execute(pool)
            .await
            .expect("principal should insert");
        }

        sqlx::query(
            "INSERT INTO users (principal_id, email, name, status, email_verified_at)
             VALUES ($1, $2, 'Target User', 'active', NOW())",
        )
        .bind(target_id)
        .bind(email)
        .execute(pool)
        .await
        .expect("user should insert");

        tenant_id
    }

    fn suspend_request(
        principal_id: Uuid,
        actor_id: Uuid,
        role: &str,
        confirm_code: &str,
        reason: &str,
    ) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri(format!("/admin/users/{principal_id}/suspend"))
            .header("content-type", "application/json")
            .header("idempotency-key", format!("test-{}", Uuid::new_v4()))
            .header("x-nvbes-actor-principal-id", actor_id.to_string())
            .header("x-nvbes-backoffice-role", role)
            .body(Body::from(
                json!({
                    "confirm_code": confirm_code,
                    "reason": reason
                })
                .to_string(),
            ))
            .expect("request should build")
    }
}
