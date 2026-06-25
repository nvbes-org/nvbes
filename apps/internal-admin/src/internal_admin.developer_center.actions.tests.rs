use axum::{Router, http::StatusCode};
use tower::ServiceExt;
use uuid::Uuid;

use crate::developer_center_actions_test_support::{
    assert_developer_audit_metadata, client_is_revoked, developer_action_count,
    developer_actions_schema_exists, developer_audit_count, idempotency_schema_exists,
    response_json, revoke_request, revoke_request_with_key, seed_workspace_actor_and_client,
    test_pool,
};

#[tokio::test]
async fn revoke_client_route_enforces_role_confirmation_grant_and_audits_success() {
    let Some(pool) = test_pool().await else {
        eprintln!("skipping test: Postgres is not reachable");
        return;
    };
    if !developer_actions_schema_exists(&pool).await {
        eprintln!("skipping test: developer action schema is missing");
        return;
    }
    if !crate::test_operator_grants::operator_grants_schema_exists(&pool).await {
        eprintln!("skipping test: operator grant schema is missing");
        return;
    }

    let actor_id = Uuid::new_v4();
    let client_id = format!("client_{}", Uuid::new_v4());
    let (tenant_id, workspace_id) =
        seed_workspace_actor_and_client(&pool, actor_id, &client_id).await;
    let app = Router::new()
        .merge(crate::developer_center_actions::router())
        .with_state(crate::app::AppState::new(
            nvbes_core::config::AppConfig::default(),
            pool.clone(),
        ));

    let denied = app
        .clone()
        .oneshot(revoke_request(
            workspace_id,
            actor_id,
            "viewer",
            &crate::backoffice_authorization::strong_confirmation_code_for_value(
                "REVOKE CLIENT",
                &client_id,
            ),
            &client_id,
            "ticket DEV-123 approved",
        ))
        .await
        .expect("route should respond");
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);

    for role in ["finance_admin", "product_admin", "security_admin"] {
        let denied = app
            .clone()
            .oneshot(revoke_request(
                workspace_id,
                actor_id,
                role,
                &crate::backoffice_authorization::strong_confirmation_code_for_value(
                    "REVOKE CLIENT",
                    &client_id,
                ),
                &client_id,
                "ticket DEV-123 approved",
            ))
            .await
            .expect("route should respond");
        assert_eq!(denied.status(), StatusCode::FORBIDDEN, "{role}");
    }

    let wrong_confirmation = app
        .clone()
        .oneshot(revoke_request(
            workspace_id,
            actor_id,
            "developer_admin",
            "REVOKE",
            &client_id,
            "ticket DEV-123 approved",
        ))
        .await
        .expect("route should respond");
    assert_eq!(wrong_confirmation.status(), StatusCode::BAD_REQUEST);

    let generic_confirmation = app
        .clone()
        .oneshot(revoke_request(
            workspace_id,
            actor_id,
            "developer_admin",
            "REVOKE CLIENT",
            &client_id,
            "ticket DEV-123 approved",
        ))
        .await
        .expect("route should respond");
    assert_eq!(generic_confirmation.status(), StatusCode::BAD_REQUEST);

    let missing_grant = app
        .clone()
        .oneshot(revoke_request(
            workspace_id,
            actor_id,
            "developer_admin",
            &crate::backoffice_authorization::strong_confirmation_code_for_value(
                "REVOKE CLIENT",
                &client_id,
            ),
            &client_id,
            "ticket DEV-123 approved",
        ))
        .await
        .expect("route should respond");
    assert_eq!(missing_grant.status(), StatusCode::FORBIDDEN);

    crate::test_operator_grants::grant_active_operator_role(&pool, actor_id, "developer_admin")
        .await;

    let accepted = app
        .oneshot(revoke_request(
            workspace_id,
            actor_id,
            "developer_admin",
            &crate::backoffice_authorization::strong_confirmation_code_for_value(
                "REVOKE CLIENT",
                &client_id,
            ),
            &client_id,
            "ticket DEV-123 approved",
        ))
        .await
        .expect("route should respond");
    assert_eq!(accepted.status(), StatusCode::OK);
    let payload = response_json(accepted).await;
    assert_eq!(payload["action_kind"], "revoke_client");
    assert_eq!(payload["status"], "applied");

    assert!(client_is_revoked(&pool, tenant_id, &client_id).await);
    assert_eq!(
        developer_action_count(&pool, tenant_id, actor_id, &client_id).await,
        1
    );
    assert_developer_audit_metadata(&pool, tenant_id, actor_id, &payload).await;
}

#[tokio::test]
async fn revoke_client_reuses_stored_idempotent_response() {
    let Some(pool) = test_pool().await else {
        eprintln!("skipping test: Postgres is not reachable");
        return;
    };
    if !developer_actions_schema_exists(&pool).await {
        eprintln!("skipping test: developer action schema is missing");
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
    let client_id = format!("client_{}", Uuid::new_v4());
    let (tenant_id, workspace_id) =
        seed_workspace_actor_and_client(&pool, actor_id, &client_id).await;
    crate::test_operator_grants::grant_active_operator_role(&pool, actor_id, "developer_admin")
        .await;

    let state = crate::app::AppState::new(nvbes_core::config::AppConfig::default(), pool.clone());
    let app = Router::new()
        .merge(crate::developer_center_actions::router())
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::idempotency::idempotency_guard,
        ))
        .with_state(state);
    let confirm_code = crate::backoffice_authorization::strong_confirmation_code_for_value(
        "REVOKE CLIENT",
        &client_id,
    );
    let idempotency_key = format!("test-{}", Uuid::new_v4());

    let first = app
        .clone()
        .oneshot(revoke_request_with_key(
            workspace_id,
            actor_id,
            "developer_admin",
            &confirm_code,
            &client_id,
            "ticket DEV-123 approved",
            &idempotency_key,
        ))
        .await
        .expect("route should respond");
    assert_eq!(first.status(), StatusCode::OK);

    let second = app
        .oneshot(revoke_request_with_key(
            workspace_id,
            actor_id,
            "developer_admin",
            &confirm_code,
            &client_id,
            "ticket DEV-123 approved",
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

    assert!(client_is_revoked(&pool, tenant_id, &client_id).await);
    assert_eq!(
        developer_action_count(&pool, tenant_id, actor_id, &client_id).await,
        1
    );
    assert_eq!(developer_audit_count(&pool, tenant_id, actor_id).await, 1);
}
