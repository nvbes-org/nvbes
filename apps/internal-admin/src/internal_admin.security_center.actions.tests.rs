use axum::{Router, body, http::StatusCode};
use serde_json::json;
use tower::ServiceExt;
use uuid::Uuid;

use crate::security_center_actions::validate_security_action_reason;
use crate::security_center_actions_test_support::{
    idempotency_schema_exists, mfa_schema_exists, revoke_mfa_request, revoke_mfa_request_with_key,
    revoke_mfa_request_without_second_approver, seed_tenant_actor_user_and_mfa, test_pool,
};

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
    if !crate::test_operator_grants::operator_grants_schema_exists(&pool).await {
        eprintln!("skipping test: operator grant schema is missing");
        return;
    }

    let actor_id = Uuid::new_v4();
    let principal_id = Uuid::new_v4();
    let factor_id = Uuid::new_v4();
    let tenant_id = seed_tenant_actor_user_and_mfa(&pool, actor_id, principal_id, factor_id).await;
    let app = Router::new()
        .merge(crate::security_center_actions::router())
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

    let missing_dual_control = app
        .clone()
        .oneshot(revoke_mfa_request_without_second_approver(
            factor_id,
            actor_id,
            "security_admin",
            &crate::backoffice_authorization::strong_confirmation_code("REVOKE MFA", factor_id),
            "ticket SEC-789 approved",
        ))
        .await
        .expect("route should respond");
    assert_eq!(missing_dual_control.status(), StatusCode::BAD_REQUEST);

    let missing_grant = app
        .clone()
        .oneshot(revoke_mfa_request(
            factor_id,
            actor_id,
            "security_admin",
            &crate::backoffice_authorization::strong_confirmation_code("REVOKE MFA", factor_id),
            "ticket SEC-789 approved",
        ))
        .await
        .expect("route should respond");
    assert_eq!(missing_grant.status(), StatusCode::FORBIDDEN);

    crate::test_operator_grants::grant_active_operator_role(&pool, actor_id, "security_admin")
        .await;

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
    let payload: serde_json::Value = serde_json::from_slice(&body).expect("body should be json");
    assert_eq!(payload["object_id"], json!(factor_id.to_string()));
    assert_eq!(payload["principal_id"], json!(principal_id.to_string()));

    let status =
        sqlx::query_scalar::<_, String>("SELECT status::text FROM mfa_factors WHERE id = $1")
            .bind(factor_id)
            .fetch_one(&pool)
            .await
            .expect("factor should exist");
    assert_eq!(status, "revoked");

    let audit_count = security_mfa_audit_count(&pool, tenant_id, actor_id, factor_id).await;
    assert_eq!(audit_count, 1);
}

#[tokio::test]
async fn revoke_mfa_reuses_stored_idempotent_response() {
    let Some(pool) = test_pool().await else {
        eprintln!("skipping test: Postgres is not reachable");
        return;
    };
    if !mfa_schema_exists(&pool).await {
        eprintln!("skipping test: MFA schema is missing");
        return;
    }
    if !crate::test_operator_grants::operator_grants_schema_exists(&pool).await {
        eprintln!("skipping test: operator grant schema is missing");
        return;
    }
    if !idempotency_schema_exists(&pool).await {
        eprintln!("skipping test: idempotency response schema is missing");
        return;
    }

    let actor_id = Uuid::new_v4();
    let principal_id = Uuid::new_v4();
    let factor_id = Uuid::new_v4();
    let tenant_id = seed_tenant_actor_user_and_mfa(&pool, actor_id, principal_id, factor_id).await;
    crate::test_operator_grants::grant_active_operator_role(&pool, actor_id, "security_admin")
        .await;

    let state = crate::app::AppState::new(nvbes_core::config::AppConfig::default(), pool.clone());
    let app = Router::new()
        .merge(crate::security_center_actions::router())
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::idempotency::idempotency_guard,
        ))
        .with_state(state);
    let confirm_code =
        crate::backoffice_authorization::strong_confirmation_code("REVOKE MFA", factor_id);
    let idempotency_key = format!("test-{}", Uuid::new_v4());

    let first = app
        .clone()
        .oneshot(revoke_mfa_request_with_key(
            factor_id,
            actor_id,
            "security_admin",
            &confirm_code,
            "ticket SEC-789 approved",
            &idempotency_key,
        ))
        .await
        .expect("route should respond");
    assert_eq!(first.status(), StatusCode::OK);

    let second = app
        .oneshot(revoke_mfa_request_with_key(
            factor_id,
            actor_id,
            "security_admin",
            &confirm_code,
            "ticket SEC-789 approved",
            &idempotency_key,
        ))
        .await
        .expect("route should respond");
    assert_eq!(second.status(), StatusCode::OK);
    assert_eq!(
        second
            .headers()
            .get("idempotency-replayed")
            .and_then(|value| value.to_str().ok()),
        Some("true")
    );

    let status =
        sqlx::query_scalar::<_, String>("SELECT status::text FROM mfa_factors WHERE id = $1")
            .bind(factor_id)
            .fetch_one(&pool)
            .await
            .expect("factor should exist");
    assert_eq!(status, "revoked");

    let audit_count = security_mfa_audit_count(&pool, tenant_id, actor_id, factor_id).await;
    assert_eq!(audit_count, 1);
}

async fn security_mfa_audit_count(
    pool: &sqlx::PgPool,
    tenant_id: Uuid,
    actor_id: Uuid,
    factor_id: Uuid,
) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM audit_events
         WHERE tenant_id = $1 AND actor_principal_id = $2
           AND action = 'internal_admin.security.mfa_factor.revoked'
           AND target_type = 'mfa_factor'
           AND target_id = $3",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .bind(factor_id)
    .fetch_one(pool)
    .await
    .expect("audit count should load")
}
