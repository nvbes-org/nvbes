use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::post,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::app::AppState;
use crate::backoffice_authorization::{
    BackofficePermission, require_idempotency_key, require_permission, require_strong_confirmation,
};
use crate::billing_admin_access::actor_principal_id;
use crate::error::AppError;

#[derive(Debug, Deserialize)]
struct SecurityActionRequest {
    confirm_code: String,
    reason: String,
}

#[derive(Debug, Serialize)]
struct SecurityActionResult {
    object_id: Uuid,
    tenant_id: Uuid,
    principal_id: Uuid,
    audit_action: &'static str,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/admin/security-center/mfa-factors/{factorId}/revoke",
            post(revoke_mfa_factor_route),
        )
        .route(
            "/admin/security-center/oauth-consents/{consentId}/revoke",
            post(revoke_oauth_consent_route),
        )
}

async fn revoke_mfa_factor_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(factor_id): Path<Uuid>,
    Json(request): Json<SecurityActionRequest>,
) -> Result<Json<SecurityActionResult>, AppError> {
    require_idempotency_key(&headers)?;
    require_permission(&headers, BackofficePermission::SecurityMutate)?;
    require_strong_confirmation(&request.confirm_code, "REVOKE MFA", factor_id)?;
    let actor_id = actor_principal_id(&headers)?;
    Ok(Json(
        revoke_mfa_factor(&state.db, actor_id, factor_id, request).await?,
    ))
}

async fn revoke_oauth_consent_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(consent_id): Path<Uuid>,
    Json(request): Json<SecurityActionRequest>,
) -> Result<Json<SecurityActionResult>, AppError> {
    require_idempotency_key(&headers)?;
    require_permission(&headers, BackofficePermission::SecurityMutate)?;
    require_strong_confirmation(&request.confirm_code, "REVOKE OAUTH CONSENT", consent_id)?;
    let actor_id = actor_principal_id(&headers)?;
    Ok(Json(
        revoke_oauth_consent(&state.db, actor_id, consent_id, request).await?,
    ))
}

async fn revoke_mfa_factor(
    db: &PgPool,
    actor_id: Uuid,
    factor_id: Uuid,
    request: SecurityActionRequest,
) -> Result<SecurityActionResult, AppError> {
    validate_security_action_reason(&request.reason)?;

    let mut tx = db.begin().await?;
    let row = sqlx::query(
        r#"
        SELECT mf.principal_id, mf.status::text AS status, p.tenant_id
        FROM mfa_factors mf
        JOIN principals p ON p.id = mf.principal_id
        WHERE mf.id = $1
        FOR UPDATE OF mf
        "#,
    )
    .bind(factor_id)
    .fetch_one(tx.as_mut())
    .await?;
    let principal_id: Uuid = row.get("principal_id");
    let tenant_id: Uuid = row.get("tenant_id");
    let status: String = row.get("status");
    if status == "revoked" {
        return Err(AppError::bad_request(
            "mfa_factor_already_revoked",
            "MFA factor is already revoked.",
        ));
    }

    sqlx::query("UPDATE mfa_factors SET status = 'revoked' WHERE id = $1")
        .bind(factor_id)
        .execute(tx.as_mut())
        .await?;
    insert_security_audit(
        tx.as_mut(),
        tenant_id,
        actor_id,
        "internal_admin.security.mfa_factor.revoked",
        "mfa_factor",
        factor_id,
        principal_id,
        &request.reason,
    )
    .await?;
    tx.commit().await?;
    Ok(SecurityActionResult {
        object_id: factor_id,
        tenant_id,
        principal_id,
        audit_action: "internal_admin.security.mfa_factor.revoked",
    })
}

async fn revoke_oauth_consent(
    db: &PgPool,
    actor_id: Uuid,
    consent_id: Uuid,
    request: SecurityActionRequest,
) -> Result<SecurityActionResult, AppError> {
    validate_security_action_reason(&request.reason)?;

    let mut tx = db.begin().await?;
    let row = sqlx::query(
        "SELECT tenant_id, principal_id, revoked_at FROM oauth_consents WHERE id = $1 FOR UPDATE",
    )
    .bind(consent_id)
    .fetch_one(tx.as_mut())
    .await?;
    let tenant_id: Uuid = row.get("tenant_id");
    let principal_id: Uuid = row.get("principal_id");
    let revoked_at: Option<DateTime<Utc>> = row.get("revoked_at");
    if revoked_at.is_some() {
        return Err(AppError::bad_request(
            "oauth_consent_already_revoked",
            "OAuth consent is already revoked.",
        ));
    }

    sqlx::query("UPDATE oauth_consents SET revoked_at = NOW() WHERE id = $1")
        .bind(consent_id)
        .execute(tx.as_mut())
        .await?;
    insert_security_audit(
        tx.as_mut(),
        tenant_id,
        actor_id,
        "internal_admin.security.oauth_consent.revoked",
        "oauth_consent",
        consent_id,
        principal_id,
        &request.reason,
    )
    .await?;
    tx.commit().await?;
    Ok(SecurityActionResult {
        object_id: consent_id,
        tenant_id,
        principal_id,
        audit_action: "internal_admin.security.oauth_consent.revoked",
    })
}

async fn insert_security_audit(
    executor: &mut sqlx::PgConnection,
    tenant_id: Uuid,
    actor_id: Uuid,
    action: &'static str,
    target_type: &str,
    target_id: Uuid,
    principal_id: Uuid,
    reason: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO audit_events (
           tenant_id, actor_principal_id, action, target_type, target_id, metadata, event_hash
         ) VALUES (
           $1, $2, $3, $4, $5,
           jsonb_build_object('reason', $6, 'principal_id', $7::text),
           gen_random_uuid()::text
         )",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .bind(action)
    .bind(target_type)
    .bind(target_id)
    .bind(reason.trim())
    .bind(principal_id)
    .execute(executor)
    .await?;
    Ok(())
}

fn validate_security_action_reason(reason: &str) -> Result<(), AppError> {
    if reason.trim().len() < 12 {
        return Err(AppError::bad_request(
            "audit_reason_required",
            "Security actions require a detailed audit reason.",
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
    fn security_action_reason_must_be_detailed() {
        assert!(validate_security_action_reason("short").is_err());
        assert!(validate_security_action_reason("incident SEC-456 approved").is_ok());
    }

    #[tokio::test]
    async fn revoke_mfa_route_enforces_role_confirmation_and_audits_success() {
        let Some(pool) = test_pool().await else {
            eprintln!("skipping test: Postgres is not reachable");
            return;
        };
        if !mfa_schema_exists(&pool).await {
            eprintln!("skipping test: MFA schema is missing");
            return;
        }

        let actor_id = Uuid::new_v4();
        let principal_id = Uuid::new_v4();
        let factor_id = Uuid::new_v4();
        let tenant_id =
            seed_tenant_actor_user_and_mfa(&pool, actor_id, principal_id, factor_id).await;
        let app = Router::new()
            .merge(router())
            .with_state(crate::app::AppState::new(
                nvbes_core::config::AppConfig::default(),
                pool.clone(),
            ));

        let denied = app
            .clone()
            .oneshot(revoke_mfa_request(
                factor_id,
                actor_id,
                "finance_admin",
                &crate::backoffice_authorization::strong_confirmation_code("REVOKE MFA", factor_id),
                "ticket SEC-789 approved",
            ))
            .await
            .expect("route should respond");
        assert_eq!(denied.status(), StatusCode::FORBIDDEN);

        let wrong_confirmation = app
            .clone()
            .oneshot(revoke_mfa_request(
                factor_id,
                actor_id,
                "security_admin",
                "REVOKE",
                "ticket SEC-789 approved",
            ))
            .await
            .expect("route should respond");
        assert_eq!(wrong_confirmation.status(), StatusCode::BAD_REQUEST);

        let generic_confirmation = app
            .clone()
            .oneshot(revoke_mfa_request(
                factor_id,
                actor_id,
                "security_admin",
                "REVOKE MFA",
                "ticket SEC-789 approved",
            ))
            .await
            .expect("route should respond");
        assert_eq!(generic_confirmation.status(), StatusCode::BAD_REQUEST);

        let accepted = app
            .oneshot(revoke_mfa_request(
                factor_id,
                actor_id,
                "security_admin",
                &crate::backoffice_authorization::strong_confirmation_code("REVOKE MFA", factor_id),
                "ticket SEC-789 approved",
            ))
            .await
            .expect("route should respond");
        assert_eq!(accepted.status(), StatusCode::OK);
        let body = body::to_bytes(accepted.into_body(), usize::MAX)
            .await
            .expect("body should be readable");
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("body should be json");
        assert_eq!(payload["object_id"], json!(factor_id.to_string()));
        assert_eq!(payload["principal_id"], json!(principal_id.to_string()));

        let status =
            sqlx::query_scalar::<_, String>("SELECT status::text FROM mfa_factors WHERE id = $1")
                .bind(factor_id)
                .fetch_one(&pool)
                .await
                .expect("factor should exist");
        assert_eq!(status, "revoked");

        let audit_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM audit_events
             WHERE tenant_id = $1 AND actor_principal_id = $2
               AND action = 'internal_admin.security.mfa_factor.revoked'
               AND target_type = 'mfa_factor'
               AND target_id = $3",
        )
        .bind(tenant_id)
        .bind(actor_id)
        .bind(factor_id)
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

    async fn mfa_schema_exists(pool: &PgPool) -> bool {
        sqlx::query_scalar::<_, bool>(
            "SELECT to_regclass('public.tenants') IS NOT NULL
              AND to_regclass('public.principals') IS NOT NULL
              AND to_regclass('public.users') IS NOT NULL
              AND to_regclass('public.mfa_factors') IS NOT NULL
              AND to_regclass('public.audit_events') IS NOT NULL",
        )
        .fetch_one(pool)
        .await
        .unwrap_or(false)
    }

    async fn seed_tenant_actor_user_and_mfa(
        pool: &PgPool,
        actor_id: Uuid,
        principal_id: Uuid,
        factor_id: Uuid,
    ) -> Uuid {
        let tenant_id = Uuid::new_v4();
        let slug = format!("test-security-tenant-{}", Uuid::new_v4());
        let email = format!("{}@example.test", Uuid::new_v4());
        sqlx::query(
            "INSERT INTO tenants (id, kind, name, slug, status, security_tier)
             VALUES ($1, 'enterprise', 'Test Security Tenant', $2, 'active', 'standard')",
        )
        .bind(tenant_id)
        .bind(slug)
        .execute(pool)
        .await
        .expect("tenant should insert");

        for (id, display_name) in [
            (actor_id, "Backoffice Actor"),
            (principal_id, "Target User"),
        ] {
            sqlx::query(
                "INSERT INTO principals (id, tenant_id, principal_kind, status, display_name)
                 VALUES ($1, $2, 'human', 'active', $3)",
            )
            .bind(id)
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
        .bind(principal_id)
        .bind(email)
        .execute(pool)
        .await
        .expect("user should insert");

        sqlx::query(
            "INSERT INTO mfa_factors (
               id, principal_id, factor_type, status, label, confirmed_at
             ) VALUES ($1, $2, 'totp', 'active', 'Test TOTP', NOW())",
        )
        .bind(factor_id)
        .bind(principal_id)
        .execute(pool)
        .await
        .expect("MFA factor should insert");

        tenant_id
    }

    fn revoke_mfa_request(
        factor_id: Uuid,
        actor_id: Uuid,
        role: &str,
        confirm_code: &str,
        reason: &str,
    ) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri(format!(
                "/admin/security-center/mfa-factors/{factor_id}/revoke"
            ))
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
