use axum::{Router, body, http::StatusCode};
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;
use uuid::Uuid;

use crate::risk_decision_center_actions_test_support::{
    block_policy_request, idempotency_schema_exists, resolve_request, resolve_request_with_key,
    resolve_request_without_second_approver, risk_action_count, risk_actions_schema_exists,
    risk_audit_count, seed_risk_signal_with_actor, test_pool,
};

#[tokio::test]
async fn block_risk_policy_rejects_generic_confirmation_code() {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://localhost/internal_admin_risk_confirmation_test")
        .expect("lazy pool should build");
    let app = Router::new()
        .merge(crate::risk_decision_center_actions::router())
        .with_state(crate::app::AppState::new(
            nvbes_core::config::AppConfig::default(),
            pool,
        ));

    let response = app
        .oneshot(block_policy_request(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            "security_admin",
            "BLOCK RISK POLICY",
        ))
        .await
        .expect("route should respond");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn resolve_risk_signal_route_enforces_grant_confirmation_dual_control_and_audits_success() {
    let Some(pool) = test_pool().await else {
        eprintln!("skipping test: Postgres is not reachable");
        return;
    };
    if !risk_actions_schema_exists(&pool).await {
        eprintln!("skipping test: risk action schema is missing");
        return;
    }
    if !crate::test_operator_grants::operator_grants_schema_exists(&pool).await {
        eprintln!("skipping test: operator grant schema is missing");
        return;
    }

    let actor_id = Uuid::new_v4();
    let (tenant_id, workspace_id, signal_id) = seed_risk_signal_with_actor(&pool, actor_id).await;
    let strong_code =
        crate::backoffice_authorization::strong_confirmation_code("RESOLVE RISK SIGNAL", signal_id);
    let app = Router::new()
        .merge(crate::risk_decision_center_actions::router())
        .with_state(crate::app::AppState::new(
            nvbes_core::config::AppConfig::default(),
            pool.clone(),
        ));

    let denied = app
        .clone()
        .oneshot(resolve_request(
            workspace_id,
            signal_id,
            actor_id,
            "viewer",
            &strong_code,
        ))
        .await
        .expect("route should respond");
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);

    for role in ["finance_admin", "operations_admin", "compliance_admin"] {
        let denied = app
            .clone()
            .oneshot(resolve_request(
                workspace_id,
                signal_id,
                actor_id,
                role,
                &strong_code,
            ))
            .await
            .expect("route should respond");
        assert_eq!(denied.status(), StatusCode::FORBIDDEN, "{role}");
    }

    let wrong_confirmation = app
        .clone()
        .oneshot(resolve_request(
            workspace_id,
            signal_id,
            actor_id,
            "security_admin",
            "RESOLVE SIGNAL",
        ))
        .await
        .expect("route should respond");
    assert_eq!(wrong_confirmation.status(), StatusCode::BAD_REQUEST);

    let generic_confirmation = app
        .clone()
        .oneshot(resolve_request(
            workspace_id,
            signal_id,
            actor_id,
            "security_admin",
            "RESOLVE RISK SIGNAL",
        ))
        .await
        .expect("route should respond");
    assert_eq!(generic_confirmation.status(), StatusCode::BAD_REQUEST);

    let missing_dual_control = app
        .clone()
        .oneshot(resolve_request_without_second_approver(
            workspace_id,
            signal_id,
            actor_id,
            "security_admin",
            &strong_code,
        ))
        .await
        .expect("route should respond");
    assert_eq!(missing_dual_control.status(), StatusCode::BAD_REQUEST);

    let missing_grant = app
        .clone()
        .oneshot(resolve_request(
            workspace_id,
            signal_id,
            actor_id,
            "security_admin",
            &strong_code,
        ))
        .await
        .expect("route should respond");
    assert_eq!(missing_grant.status(), StatusCode::FORBIDDEN);

    crate::test_operator_grants::grant_active_operator_role(&pool, actor_id, "security_admin")
        .await;

    let accepted = app
        .oneshot(resolve_request(
            workspace_id,
            signal_id,
            actor_id,
            "security_admin",
            &strong_code,
        ))
        .await
        .expect("route should respond");
    assert_eq!(accepted.status(), StatusCode::OK);
    let body = body::to_bytes(accepted.into_body(), usize::MAX)
        .await
        .expect("body should be readable");
    let payload: serde_json::Value = serde_json::from_slice(&body).expect("body should be json");
    assert_eq!(payload["action_kind"], json!("resolve_risk_signal"));
    assert_eq!(payload["status"], json!("resolved"));

    assert_risk_signal_resolved(&pool, signal_id).await;
    assert_eq!(
        risk_action_count(&pool, tenant_id, actor_id, signal_id).await,
        1
    );
    assert_eq!(risk_audit_count(&pool, tenant_id, actor_id).await, 1);

    let audit_metadata = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT metadata FROM audit_events
         WHERE tenant_id = $1 AND actor_principal_id = $2
           AND action = 'risk.signal.resolved'
           AND target_type = 'billing_risk_signal'",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .fetch_one(&pool)
    .await
    .expect("audit metadata should load");
    assert_eq!(audit_metadata["risk_action_id"], payload["object_id"]);
    assert_eq!(
        audit_metadata["object_links"]["risk_signal_id"],
        json!(signal_id)
    );
    assert_eq!(
        audit_metadata["changes"][0],
        json!({
            "field": "resolution_status",
            "before": "unresolved",
            "after": "resolved"
        })
    );
}

#[tokio::test]
async fn resolve_risk_signal_reuses_stored_idempotent_response() {
    let Some(pool) = test_pool().await else {
        eprintln!("skipping test: Postgres is not reachable");
        return;
    };
    if !risk_actions_schema_exists(&pool).await {
        eprintln!("skipping test: risk action schema is missing");
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
    let (tenant_id, workspace_id, signal_id) = seed_risk_signal_with_actor(&pool, actor_id).await;
    crate::test_operator_grants::grant_active_operator_role(&pool, actor_id, "security_admin")
        .await;

    let state = crate::app::AppState::new(nvbes_core::config::AppConfig::default(), pool.clone());
    let app = Router::new()
        .merge(crate::risk_decision_center_actions::router())
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::idempotency::idempotency_guard,
        ))
        .with_state(state);
    let confirm_code =
        crate::backoffice_authorization::strong_confirmation_code("RESOLVE RISK SIGNAL", signal_id);
    let idempotency_key = format!("test-{}", Uuid::new_v4());

    let first = app
        .clone()
        .oneshot(resolve_request_with_key(
            workspace_id,
            signal_id,
            actor_id,
            "security_admin",
            &confirm_code,
            &idempotency_key,
        ))
        .await
        .expect("route should respond");
    assert_eq!(first.status(), StatusCode::OK);

    let second = app
        .oneshot(resolve_request_with_key(
            workspace_id,
            signal_id,
            actor_id,
            "security_admin",
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

    assert_risk_signal_resolved(&pool, signal_id).await;
    assert_eq!(
        risk_action_count(&pool, tenant_id, actor_id, signal_id).await,
        1
    );
    assert_eq!(risk_audit_count(&pool, tenant_id, actor_id).await, 1);
}

async fn assert_risk_signal_resolved(pool: &sqlx::PgPool, signal_id: Uuid) {
    let resolution = sqlx::query_scalar::<_, Option<String>>(
        "SELECT signal_value->>'resolution_status' FROM billing_risk_signals WHERE id = $1",
    )
    .bind(signal_id)
    .fetch_one(pool)
    .await
    .expect("risk signal should load");
    assert_eq!(resolution.as_deref(), Some("resolved"));
}
