use axum::{Router, http::StatusCode};
use tower::ServiceExt;
use uuid::Uuid;

use crate::communications_center_actions_test_support::{
    assert_communications_audit_metadata, communications_action_count,
    communications_actions_schema_exists, communications_audit_count, idempotency_schema_exists,
    response_json, seed_workspace_with_actor, suppress_request, suppress_request_with_key,
    test_pool,
};

#[tokio::test]
async fn suppress_email_route_enforces_role_confirmation_grant_and_audits_success() {
    let Some(pool) = test_pool().await else {
        eprintln!("skipping test: Postgres is not reachable");
        return;
    };
    if !communications_actions_schema_exists(&pool).await {
        eprintln!("skipping test: communications action schema is missing");
        return;
    }
    if !crate::test_operator_grants::operator_grants_schema_exists(&pool).await {
        eprintln!("skipping test: operator grant schema is missing");
        return;
    }

    let actor_id = Uuid::new_v4();
    let email = format!("{}@example.com", Uuid::new_v4());
    let (tenant_id, workspace_id) = seed_workspace_with_actor(&pool, actor_id).await;
    let app = Router::new()
        .merge(crate::communications_center_actions::router())
        .with_state(crate::app::AppState::new(
            nvbes_core::config::AppConfig::default(),
            pool.clone(),
        ));

    let denied = app
        .clone()
        .oneshot(suppress_request(
            workspace_id,
            actor_id,
            "viewer",
            &crate::backoffice_authorization::strong_confirmation_code(
                "SUPPRESS EMAIL",
                workspace_id,
            ),
            &email,
            "ticket COMMS-123 approved",
        ))
        .await
        .expect("route should respond");
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);

    for role in ["finance_admin", "developer_admin", "product_admin"] {
        let denied = app
            .clone()
            .oneshot(suppress_request(
                workspace_id,
                actor_id,
                role,
                &crate::backoffice_authorization::strong_confirmation_code(
                    "SUPPRESS EMAIL",
                    workspace_id,
                ),
                &email,
                "ticket COMMS-123 approved",
            ))
            .await
            .expect("route should respond");
        assert_eq!(denied.status(), StatusCode::FORBIDDEN, "{role}");
    }

    let wrong_confirmation = app
        .clone()
        .oneshot(suppress_request(
            workspace_id,
            actor_id,
            "operations_admin",
            "SUPPRESS",
            &email,
            "ticket COMMS-123 approved",
        ))
        .await
        .expect("route should respond");
    assert_eq!(wrong_confirmation.status(), StatusCode::BAD_REQUEST);

    let generic_confirmation = app
        .clone()
        .oneshot(suppress_request(
            workspace_id,
            actor_id,
            "operations_admin",
            "SUPPRESS EMAIL",
            &email,
            "ticket COMMS-123 approved",
        ))
        .await
        .expect("route should respond");
    assert_eq!(generic_confirmation.status(), StatusCode::BAD_REQUEST);

    let missing_grant = app
        .clone()
        .oneshot(suppress_request(
            workspace_id,
            actor_id,
            "operations_admin",
            &crate::backoffice_authorization::strong_confirmation_code(
                "SUPPRESS EMAIL",
                workspace_id,
            ),
            &email,
            "ticket COMMS-123 approved",
        ))
        .await
        .expect("route should respond");
    assert_eq!(missing_grant.status(), StatusCode::FORBIDDEN);

    crate::test_operator_grants::grant_active_operator_role(&pool, actor_id, "operations_admin")
        .await;

    let accepted = app
        .oneshot(suppress_request(
            workspace_id,
            actor_id,
            "operations_admin",
            &crate::backoffice_authorization::strong_confirmation_code(
                "SUPPRESS EMAIL",
                workspace_id,
            ),
            &email,
            "ticket COMMS-123 approved",
        ))
        .await
        .expect("route should respond");
    assert_eq!(accepted.status(), StatusCode::OK);
    let payload = response_json(accepted).await;
    assert_eq!(payload["action_kind"], "suppress_email");
    assert_eq!(payload["status"], "applied");

    assert_eq!(
        communications_action_count(&pool, tenant_id, actor_id, &email).await,
        1
    );
    assert_communications_audit_metadata(&pool, tenant_id, actor_id, &email, &payload).await;
}

#[tokio::test]
async fn suppress_email_reuses_stored_idempotent_response() {
    let Some(pool) = test_pool().await else {
        eprintln!("skipping test: Postgres is not reachable");
        return;
    };
    if !communications_actions_schema_exists(&pool).await {
        eprintln!("skipping test: communications action schema is missing");
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
    let email = format!("{}@example.com", Uuid::new_v4());
    let (tenant_id, workspace_id) = seed_workspace_with_actor(&pool, actor_id).await;
    crate::test_operator_grants::grant_active_operator_role(&pool, actor_id, "operations_admin")
        .await;

    let state = crate::app::AppState::new(nvbes_core::config::AppConfig::default(), pool.clone());
    let app = Router::new()
        .merge(crate::communications_center_actions::router())
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::idempotency::idempotency_guard,
        ))
        .with_state(state);
    let confirm_code =
        crate::backoffice_authorization::strong_confirmation_code("SUPPRESS EMAIL", workspace_id);
    let idempotency_key = format!("test-{}", Uuid::new_v4());

    let first = app
        .clone()
        .oneshot(suppress_request_with_key(
            workspace_id,
            actor_id,
            "operations_admin",
            &confirm_code,
            &email,
            "ticket COMMS-123 approved",
            &idempotency_key,
        ))
        .await
        .expect("route should respond");
    assert_eq!(first.status(), StatusCode::OK);

    let second = app
        .oneshot(suppress_request_with_key(
            workspace_id,
            actor_id,
            "operations_admin",
            &confirm_code,
            &email,
            "ticket COMMS-123 approved",
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
        communications_action_count(&pool, tenant_id, actor_id, &email).await,
        1
    );
    assert_eq!(
        communications_audit_count(&pool, tenant_id, actor_id).await,
        1
    );
}
