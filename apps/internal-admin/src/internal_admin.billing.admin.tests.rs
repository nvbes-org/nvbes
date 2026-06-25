use axum::{Router, body, http::StatusCode};
use serde_json::json;
use tower::ServiceExt;
use uuid::Uuid;

use crate::billing_admin_test_support::{
    billing_replay_schema_exists, idempotency_schema_exists, provider_event_status,
    replay_audit_count, replay_request, replay_request_with_key,
    seed_workspace_actor_and_provider_event, test_pool,
};

#[tokio::test]
async fn replay_provider_event_route_enforces_grant_confirmation_and_audits_success() {
    let Some(pool) = test_pool().await else {
        eprintln!("skipping test: Postgres is not reachable");
        return;
    };
    if !billing_replay_schema_exists(&pool).await {
        eprintln!("skipping test: billing replay schema is missing");
        return;
    }
    if !crate::test_operator_grants::operator_grants_schema_exists(&pool).await {
        eprintln!("skipping test: operator grant schema is missing");
        return;
    }

    let actor_id = Uuid::new_v4();
    let approver_id = Uuid::new_v4();
    let provider_event_id = format!("evt_{}", Uuid::new_v4());
    let strong_code = crate::backoffice_authorization::strong_confirmation_code_for_value(
        "REPLAY EVENT",
        &provider_event_id,
    );
    let (tenant_id, workspace_id, event_id) =
        seed_workspace_actor_and_provider_event(&pool, actor_id, &provider_event_id).await;
    let app = Router::new()
        .merge(crate::billing_admin::router())
        .with_state(crate::app::AppState::new(
            nvbes_core::config::AppConfig::default(),
            pool.clone(),
        ));

    let denied = app
        .clone()
        .oneshot(replay_request(
            workspace_id,
            actor_id,
            Some(approver_id),
            "viewer",
            &strong_code,
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
            Some(approver_id),
            "finance_admin",
            "REPLAY",
            &provider_event_id,
            "ticket BILL-456 approved",
        ))
        .await
        .expect("route should respond");
    assert_eq!(wrong_confirmation.status(), StatusCode::BAD_REQUEST);

    let missing_dual_control = app
        .clone()
        .oneshot(replay_request(
            workspace_id,
            actor_id,
            None,
            "finance_admin",
            &strong_code,
            &provider_event_id,
            "ticket BILL-456 approved",
        ))
        .await
        .expect("route should respond");
    assert_eq!(missing_dual_control.status(), StatusCode::BAD_REQUEST);

    let missing_grant = app
        .clone()
        .oneshot(replay_request(
            workspace_id,
            actor_id,
            Some(approver_id),
            "finance_admin",
            &strong_code,
            &provider_event_id,
            "ticket BILL-456 approved",
        ))
        .await
        .expect("route should respond");
    assert_eq!(missing_grant.status(), StatusCode::FORBIDDEN);

    crate::test_operator_grants::grant_active_operator_role(&pool, actor_id, "finance_admin").await;

    let accepted = app
        .oneshot(replay_request(
            workspace_id,
            actor_id,
            Some(approver_id),
            "finance_admin",
            &strong_code,
            &provider_event_id,
            "ticket BILL-456 approved",
        ))
        .await
        .expect("route should respond");
    assert_eq!(accepted.status(), StatusCode::OK);
    let body = body::to_bytes(accepted.into_body(), usize::MAX)
        .await
        .expect("body should be readable");
    let payload: serde_json::Value = serde_json::from_slice(&body).expect("body should be json");
    assert_eq!(payload["object_id"], json!(event_id.to_string()));
    assert_eq!(payload["status"], json!("replayed"));
    assert_eq!(provider_event_status(&pool, event_id).await, "replayed");
    assert_eq!(
        replay_audit_count(&pool, tenant_id, actor_id, event_id).await,
        1
    );
}

#[tokio::test]
async fn replay_provider_event_reuses_stored_idempotent_response() {
    let Some(pool) = test_pool().await else {
        eprintln!("skipping test: Postgres is not reachable");
        return;
    };
    if !billing_replay_schema_exists(&pool).await {
        eprintln!("skipping test: billing replay schema is missing");
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
    let approver_id = Uuid::new_v4();
    let provider_event_id = format!("evt_{}", Uuid::new_v4());
    let strong_code = crate::backoffice_authorization::strong_confirmation_code_for_value(
        "REPLAY EVENT",
        &provider_event_id,
    );
    let (tenant_id, workspace_id, event_id) =
        seed_workspace_actor_and_provider_event(&pool, actor_id, &provider_event_id).await;
    crate::test_operator_grants::grant_active_operator_role(&pool, actor_id, "finance_admin").await;

    let state = crate::app::AppState::new(nvbes_core::config::AppConfig::default(), pool.clone());
    let app = Router::new()
        .merge(crate::billing_admin::router())
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::idempotency::idempotency_guard,
        ))
        .with_state(state);
    let idempotency_key = format!("test-{}", Uuid::new_v4());

    let first = app
        .clone()
        .oneshot(replay_request_with_key(
            workspace_id,
            actor_id,
            Some(approver_id),
            "finance_admin",
            &strong_code,
            &provider_event_id,
            "ticket BILL-456 approved",
            &idempotency_key,
        ))
        .await
        .expect("route should respond");
    assert_eq!(first.status(), StatusCode::OK);

    let second = app
        .oneshot(replay_request_with_key(
            workspace_id,
            actor_id,
            Some(approver_id),
            "finance_admin",
            &strong_code,
            &provider_event_id,
            "ticket BILL-456 approved",
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
    assert_eq!(provider_event_status(&pool, event_id).await, "replayed");
    assert_eq!(
        replay_audit_count(&pool, tenant_id, actor_id, event_id).await,
        1
    );
}
