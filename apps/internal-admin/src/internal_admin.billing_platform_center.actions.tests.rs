use axum::{Router, http::StatusCode};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;
use uuid::Uuid;

use crate::billing_platform_center_actions_test_support::{
    approve_audit_count, approve_request, approve_request_with_key, assert_approve_audit_metadata,
    assert_approved_kyc_state, billing_platform_actions_schema_exists, disable_routing_request,
    idempotency_schema_exists, platform_action_count, reject_request, response_json,
    seed_kyc_profile_with_actor, test_pool,
};

#[tokio::test]
async fn reject_kyc_profile_rejects_generic_confirmation_code() {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://localhost/internal_admin_billing_platform_confirmation_test")
        .expect("lazy pool should build");
    let app = Router::new()
        .merge(crate::billing_platform_center_actions::router())
        .with_state(crate::app::AppState::new(
            nvbes_core::config::AppConfig::default(),
            pool,
        ));

    let response = app
        .oneshot(reject_request(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            "finance_admin",
            "REJECT KYC",
        ))
        .await
        .expect("route should respond");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn disable_routing_rule_rejects_generic_confirmation_code() {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://localhost/internal_admin_billing_platform_confirmation_test")
        .expect("lazy pool should build");
    let app = Router::new()
        .merge(crate::billing_platform_center_actions::router())
        .with_state(crate::app::AppState::new(
            nvbes_core::config::AppConfig::default(),
            pool,
        ));

    let response = app
        .oneshot(disable_routing_request(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            "finance_admin",
            "DISABLE ROUTING RULE",
        ))
        .await
        .expect("route should respond");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn approve_kyc_profile_route_enforces_role_confirmation_grant_and_audits_success() {
    let Some(pool) = test_pool().await else {
        eprintln!("skipping test: Postgres is not reachable");
        return;
    };
    if !billing_platform_actions_schema_exists(&pool).await {
        eprintln!("skipping test: billing platform action schema is missing");
        return;
    }
    if !crate::test_operator_grants::operator_grants_schema_exists(&pool).await {
        eprintln!("skipping test: operator grant schema is missing");
        return;
    }

    let actor_id = Uuid::new_v4();
    let (tenant_id, workspace_id, profile_id) = seed_kyc_profile_with_actor(&pool, actor_id).await;
    let app = Router::new()
        .merge(crate::billing_platform_center_actions::router())
        .with_state(crate::app::AppState::new(
            nvbes_core::config::AppConfig::default(),
            pool.clone(),
        ));

    let denied = app
        .clone()
        .oneshot(approve_request(
            workspace_id,
            profile_id,
            actor_id,
            "viewer",
            &crate::backoffice_authorization::strong_confirmation_code("APPROVE KYC", profile_id),
        ))
        .await
        .expect("route should respond");
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);

    let wrong_confirmation = app
        .clone()
        .oneshot(approve_request(
            workspace_id,
            profile_id,
            actor_id,
            "finance_admin",
            "APPROVE",
        ))
        .await
        .expect("route should respond");
    assert_eq!(wrong_confirmation.status(), StatusCode::BAD_REQUEST);

    let missing_grant = app
        .clone()
        .oneshot(approve_request(
            workspace_id,
            profile_id,
            actor_id,
            "finance_admin",
            &crate::backoffice_authorization::strong_confirmation_code("APPROVE KYC", profile_id),
        ))
        .await
        .expect("route should respond");
    assert_eq!(missing_grant.status(), StatusCode::FORBIDDEN);

    crate::test_operator_grants::grant_active_operator_role(&pool, actor_id, "finance_admin").await;

    let accepted = app
        .oneshot(approve_request(
            workspace_id,
            profile_id,
            actor_id,
            "finance_admin",
            &crate::backoffice_authorization::strong_confirmation_code("APPROVE KYC", profile_id),
        ))
        .await
        .expect("route should respond");
    assert_eq!(accepted.status(), StatusCode::OK);
    let payload = response_json(accepted).await;
    assert_eq!(payload["action_kind"], "approve_kyc_profile");
    assert_eq!(payload["status"], "approved");

    assert_approved_kyc_state(&pool, profile_id).await;
    assert_eq!(
        platform_action_count(&pool, tenant_id, actor_id, profile_id).await,
        1
    );
    assert_approve_audit_metadata(&pool, tenant_id, actor_id, profile_id, &payload).await;
}

#[tokio::test]
async fn approve_kyc_profile_reuses_stored_idempotent_response() {
    let Some(pool) = test_pool().await else {
        eprintln!("skipping test: Postgres is not reachable");
        return;
    };
    if !billing_platform_actions_schema_exists(&pool).await {
        eprintln!("skipping test: billing platform action schema is missing");
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
    let (tenant_id, workspace_id, profile_id) = seed_kyc_profile_with_actor(&pool, actor_id).await;
    crate::test_operator_grants::grant_active_operator_role(&pool, actor_id, "finance_admin").await;

    let state = crate::app::AppState::new(nvbes_core::config::AppConfig::default(), pool.clone());
    let app = Router::new()
        .merge(crate::billing_platform_center_actions::router())
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::idempotency::idempotency_guard,
        ))
        .with_state(state);
    let confirm_code =
        crate::backoffice_authorization::strong_confirmation_code("APPROVE KYC", profile_id);
    let idempotency_key = format!("test-{}", Uuid::new_v4());

    let first = app
        .clone()
        .oneshot(approve_request_with_key(
            workspace_id,
            profile_id,
            actor_id,
            "finance_admin",
            &confirm_code,
            &idempotency_key,
        ))
        .await
        .expect("route should respond");
    assert_eq!(first.status(), StatusCode::OK);

    let second = app
        .oneshot(approve_request_with_key(
            workspace_id,
            profile_id,
            actor_id,
            "finance_admin",
            &confirm_code,
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

    assert_approved_kyc_state(&pool, profile_id).await;
    assert_eq!(
        platform_action_count(&pool, tenant_id, actor_id, profile_id).await,
        1
    );
    assert_eq!(approve_audit_count(&pool, tenant_id, actor_id).await, 1);
}
