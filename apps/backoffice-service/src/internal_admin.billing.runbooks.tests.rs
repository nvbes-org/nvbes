use axum::{Router, http::StatusCode};
use serde_json::json;
use tower::ServiceExt;
use uuid::Uuid;

use crate::billing_runbooks_test_support::{
    execute_runbook_request, execute_runbook_request_with_key, idempotency_schema_exists,
    runbook_audit_count, runbook_audit_metadata, runbook_schema_exists, seed_workspace_with_actor,
    test_pool,
};

#[tokio::test]
async fn execute_runbook_requires_active_operator_grant_and_audits_success() {
    let Some(pool) = test_pool().await else {
        eprintln!("skipping test: Postgres is not reachable");
        return;
    };
    if !runbook_schema_exists(&pool).await {
        eprintln!("skipping test: runbook schema is missing");
        return;
    }
    if !crate::test_operator_grants::operator_grants_schema_exists(&pool).await {
        eprintln!("skipping test: operator grant schema is missing");
        return;
    }

    let actor_id = Uuid::new_v4();
    let (tenant_id, workspace_id) = seed_workspace_with_actor(&pool, actor_id).await;
    let runbook_id = "psp-outage";
    let confirm_code = crate::backoffice_authorization::strong_confirmation_code_for_value(
        "EXECUTE RUNBOOK",
        runbook_id,
    );
    let app = Router::new()
        .merge(crate::billing_runbooks::router())
        .with_state(crate::app::AppState::new(
            nvbes_core::config::AppConfig::default(),
            pool.clone(),
        ));

    let missing_grant = app
        .clone()
        .oneshot(execute_runbook_request(
            workspace_id,
            actor_id,
            "finance_admin",
            &confirm_code,
            runbook_id,
        ))
        .await
        .expect("route should respond");
    assert_eq!(missing_grant.status(), StatusCode::FORBIDDEN);

    crate::test_operator_grants::grant_active_operator_role(&pool, actor_id, "finance_admin").await;

    let accepted = app
        .oneshot(execute_runbook_request(
            workspace_id,
            actor_id,
            "finance_admin",
            &confirm_code,
            runbook_id,
        ))
        .await
        .expect("route should respond");
    assert_eq!(accepted.status(), StatusCode::OK);
    assert_eq!(
        runbook_audit_count(&pool, tenant_id, workspace_id, actor_id, runbook_id).await,
        1
    );
    let metadata =
        runbook_audit_metadata(&pool, tenant_id, workspace_id, actor_id, runbook_id).await;
    assert_eq!(metadata["object_links"]["runbook_id"], json!(runbook_id));
    assert_eq!(
        metadata["changes"][0],
        json!({
            "field": "runbook.execution",
            "before": null,
            "after": "recorded"
        })
    );
}

#[tokio::test]
async fn execute_runbook_reuses_stored_idempotent_response() {
    let Some(pool) = test_pool().await else {
        eprintln!("skipping test: Postgres is not reachable");
        return;
    };
    if !runbook_schema_exists(&pool).await {
        eprintln!("skipping test: runbook schema is missing");
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
    let runbook_id = "psp-outage";
    let confirm_code = crate::backoffice_authorization::strong_confirmation_code_for_value(
        "EXECUTE RUNBOOK",
        runbook_id,
    );
    crate::test_operator_grants::grant_active_operator_role(&pool, actor_id, "finance_admin").await;

    let state = crate::app::AppState::new(nvbes_core::config::AppConfig::default(), pool.clone());
    let app = Router::new()
        .merge(crate::billing_runbooks::router())
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::idempotency::idempotency_guard,
        ))
        .with_state(state);
    let idempotency_key = format!("test-{}", Uuid::new_v4());

    let first = app
        .clone()
        .oneshot(execute_runbook_request_with_key(
            workspace_id,
            actor_id,
            "finance_admin",
            &confirm_code,
            runbook_id,
            &idempotency_key,
        ))
        .await
        .expect("route should respond");
    assert_eq!(first.status(), StatusCode::OK);

    let second = app
        .oneshot(execute_runbook_request_with_key(
            workspace_id,
            actor_id,
            "finance_admin",
            &confirm_code,
            runbook_id,
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
        runbook_audit_count(&pool, tenant_id, workspace_id, actor_id, runbook_id).await,
        1
    );
}
