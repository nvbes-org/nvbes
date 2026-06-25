use axum::{Router, http::StatusCode};
use tower::ServiceExt;
use uuid::Uuid;

use crate::entitlements_center_actions_test_support::{
    assert_grant_audit_metadata, entitlement_actions_schema_exists, grant_action_count,
    grant_audit_count, grant_request, grant_request_with_key, idempotency_schema_exists,
    response_json, seed_workspace_with_actor, test_pool,
};

#[tokio::test]
async fn grant_feature_route_enforces_role_confirmation_and_audits_success() {
    let Some(pool) = test_pool().await else {
        eprintln!("skipping test: Postgres is not reachable");
        return;
    };
    if !entitlement_actions_schema_exists(&pool).await {
        eprintln!("skipping test: entitlement action schema is missing");
        return;
    }
    if !crate::test_operator_grants::operator_grants_schema_exists(&pool).await {
        eprintln!("skipping test: operator grant schema is missing");
        return;
    }

    let actor_id = Uuid::new_v4();
    let (tenant_id, workspace_id) = seed_workspace_with_actor(&pool, actor_id).await;
    let app = Router::new()
        .merge(crate::entitlements_center_actions::router())
        .with_state(crate::app::AppState::new(
            nvbes_core::config::AppConfig::default(),
            pool.clone(),
        ));

    let denied = app
        .clone()
        .oneshot(grant_request(
            workspace_id,
            actor_id,
            "viewer",
            &crate::backoffice_authorization::strong_confirmation_code_for_value(
                "GRANT FEATURE",
                "advanced_search",
            ),
            "ticket ENT-123 approved",
        ))
        .await
        .expect("route should respond");
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);

    for role in ["finance_admin", "developer_admin", "security_admin"] {
        let denied = app
            .clone()
            .oneshot(grant_request(
                workspace_id,
                actor_id,
                role,
                &crate::backoffice_authorization::strong_confirmation_code_for_value(
                    "GRANT FEATURE",
                    "advanced_search",
                ),
                "ticket ENT-123 approved",
            ))
            .await
            .expect("route should respond");
        assert_eq!(denied.status(), StatusCode::FORBIDDEN, "{role}");
    }

    let wrong_confirmation = app
        .clone()
        .oneshot(grant_request(
            workspace_id,
            actor_id,
            "product_admin",
            "GRANT",
            "ticket ENT-123 approved",
        ))
        .await
        .expect("route should respond");
    assert_eq!(wrong_confirmation.status(), StatusCode::BAD_REQUEST);

    let generic_confirmation = app
        .clone()
        .oneshot(grant_request(
            workspace_id,
            actor_id,
            "product_admin",
            "GRANT FEATURE",
            "ticket ENT-123 approved",
        ))
        .await
        .expect("route should respond");
    assert_eq!(generic_confirmation.status(), StatusCode::BAD_REQUEST);

    let missing_grant = app
        .clone()
        .oneshot(grant_request(
            workspace_id,
            actor_id,
            "product_admin",
            &crate::backoffice_authorization::strong_confirmation_code_for_value(
                "GRANT FEATURE",
                "advanced_search",
            ),
            "ticket ENT-123 approved",
        ))
        .await
        .expect("route should respond");
    assert_eq!(missing_grant.status(), StatusCode::FORBIDDEN);

    crate::test_operator_grants::grant_active_operator_role(&pool, actor_id, "product_admin").await;

    let accepted = app
        .oneshot(grant_request(
            workspace_id,
            actor_id,
            "product_admin",
            &crate::backoffice_authorization::strong_confirmation_code_for_value(
                "GRANT FEATURE",
                "advanced_search",
            ),
            "ticket ENT-123 approved",
        ))
        .await
        .expect("route should respond");
    assert_eq!(accepted.status(), StatusCode::OK);
    let payload = response_json(accepted).await;
    assert_eq!(payload["action_kind"], "grant_feature");
    assert_eq!(payload["status"], "queued");

    assert_eq!(
        grant_action_count(&pool, tenant_id, workspace_id, actor_id).await,
        1
    );
    assert_grant_audit_metadata(&pool, tenant_id, actor_id, &payload).await;
}

#[tokio::test]
async fn grant_feature_reuses_stored_idempotent_response() {
    let Some(pool) = test_pool().await else {
        eprintln!("skipping test: Postgres is not reachable");
        return;
    };
    if !entitlement_actions_schema_exists(&pool).await {
        eprintln!("skipping test: entitlement action schema is missing");
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
    let (tenant_id, workspace_id) = seed_workspace_with_actor(&pool, actor_id).await;
    crate::test_operator_grants::grant_active_operator_role(&pool, actor_id, "product_admin").await;

    let state = crate::app::AppState::new(nvbes_core::config::AppConfig::default(), pool.clone());
    let app = Router::new()
        .merge(crate::entitlements_center_actions::router())
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::idempotency::idempotency_guard,
        ))
        .with_state(state);
    let confirm_code = crate::backoffice_authorization::strong_confirmation_code_for_value(
        "GRANT FEATURE",
        "advanced_search",
    );
    let idempotency_key = format!("test-{}", Uuid::new_v4());

    let first = app
        .clone()
        .oneshot(grant_request_with_key(
            workspace_id,
            actor_id,
            "product_admin",
            &confirm_code,
            "ticket ENT-123 approved",
            &idempotency_key,
        ))
        .await
        .expect("route should respond");
    assert_eq!(first.status(), StatusCode::OK);

    let second = app
        .oneshot(grant_request_with_key(
            workspace_id,
            actor_id,
            "product_admin",
            &confirm_code,
            "ticket ENT-123 approved",
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

    assert_eq!(
        grant_action_count(&pool, tenant_id, workspace_id, actor_id).await,
        1
    );
    assert_eq!(grant_audit_count(&pool, tenant_id, actor_id).await, 1);
}
