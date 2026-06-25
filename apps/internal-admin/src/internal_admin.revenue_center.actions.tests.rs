use axum::{Router, body, http::StatusCode};
use serde_json::json;
use tower::ServiceExt;
use uuid::Uuid;

use crate::revenue_center_actions_test_support::{
    hold_request, hold_request_with_key, idempotency_schema_exists, revenue_actions_schema_exists,
    seed_invoice_with_actor, test_pool,
};

#[tokio::test]
async fn hold_invoice_route_enforces_role_confirmation_and_audits_success() {
    let Some(pool) = test_pool().await else {
        eprintln!("skipping test: Postgres is not reachable");
        return;
    };
    if !revenue_actions_schema_exists(&pool).await {
        eprintln!("skipping test: revenue action schema is missing");
        return;
    }
    if !crate::test_operator_grants::operator_grants_schema_exists(&pool).await {
        eprintln!("skipping test: operator grant schema is missing");
        return;
    }

    let actor_id = Uuid::new_v4();
    let (tenant_id, workspace_id, invoice_id) = seed_invoice_with_actor(&pool, actor_id).await;
    let app = Router::new()
        .merge(crate::revenue_center_actions::router())
        .with_state(crate::app::AppState::new(
            nvbes_core::config::AppConfig::default(),
            pool.clone(),
        ));

    let denied = app
        .clone()
        .oneshot(hold_request(
            workspace_id,
            invoice_id,
            actor_id,
            "viewer",
            "HOLD INVOICE",
        ))
        .await
        .expect("route should respond");
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);

    let wrong_confirmation = app
        .clone()
        .oneshot(hold_request(
            workspace_id,
            invoice_id,
            actor_id,
            "finance_admin",
            "HOLD",
        ))
        .await
        .expect("route should respond");
    assert_eq!(wrong_confirmation.status(), StatusCode::BAD_REQUEST);

    let generic_confirmation = app
        .clone()
        .oneshot(hold_request(
            workspace_id,
            invoice_id,
            actor_id,
            "finance_admin",
            "HOLD INVOICE",
        ))
        .await
        .expect("route should respond");
    assert_eq!(generic_confirmation.status(), StatusCode::BAD_REQUEST);

    let missing_grant = app
        .clone()
        .oneshot(hold_request(
            workspace_id,
            invoice_id,
            actor_id,
            "finance_admin",
            &crate::backoffice_authorization::strong_confirmation_code("HOLD INVOICE", invoice_id),
        ))
        .await
        .expect("route should respond");
    assert_eq!(missing_grant.status(), StatusCode::FORBIDDEN);

    crate::test_operator_grants::grant_active_operator_role(&pool, actor_id, "finance_admin").await;

    let accepted = app
        .oneshot(hold_request(
            workspace_id,
            invoice_id,
            actor_id,
            "finance_admin",
            &crate::backoffice_authorization::strong_confirmation_code("HOLD INVOICE", invoice_id),
        ))
        .await
        .expect("route should respond");
    assert_eq!(accepted.status(), StatusCode::OK);
    let body = body::to_bytes(accepted.into_body(), usize::MAX)
        .await
        .expect("body should be readable");
    let payload: serde_json::Value = serde_json::from_slice(&body).expect("body should be json");
    assert_eq!(payload["action_kind"], json!("hold_invoice"));
    assert_eq!(payload["status"], json!("held"));

    let hold_actor = sqlx::query_scalar::<_, Option<Uuid>>(
        "SELECT internal_hold_by_principal_id FROM billing_invoices WHERE id = $1",
    )
    .bind(invoice_id)
    .fetch_one(&pool)
    .await
    .expect("invoice hold actor should load");
    assert_eq!(hold_actor, Some(actor_id));

    let action_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM internal_admin_revenue_actions
         WHERE tenant_id = $1 AND actor_principal_id = $2
           AND action_kind = 'hold_invoice' AND invoice_id = $3",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .bind(invoice_id)
    .fetch_one(&pool)
    .await
    .expect("action count should load");
    assert_eq!(action_count, 1);

    let audit_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM audit_events
         WHERE tenant_id = $1 AND actor_principal_id = $2
           AND action = 'revenue.invoice.held'
           AND target_type = 'billing_invoice'",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .fetch_one(&pool)
    .await
    .expect("audit count should load");
    assert_eq!(audit_count, 1);

    let audit_metadata = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT metadata FROM audit_events
         WHERE tenant_id = $1 AND actor_principal_id = $2
           AND action = 'revenue.invoice.held'
           AND target_type = 'billing_invoice'",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .fetch_one(&pool)
    .await
    .expect("audit metadata should load");
    assert_eq!(audit_metadata["revenue_action_id"], payload["object_id"]);
    assert_eq!(
        audit_metadata["object_links"]["invoice_id"],
        json!(invoice_id)
    );
    assert_eq!(
        audit_metadata["changes"][0],
        json!({
            "field": "internal_hold",
            "before": false,
            "after": true
        })
    );
}

#[tokio::test]
async fn hold_invoice_reuses_stored_idempotent_response() {
    let Some(pool) = test_pool().await else {
        eprintln!("skipping test: Postgres is not reachable");
        return;
    };
    if !revenue_actions_schema_exists(&pool).await {
        eprintln!("skipping test: revenue action schema is missing");
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
    let (tenant_id, workspace_id, invoice_id) = seed_invoice_with_actor(&pool, actor_id).await;
    crate::test_operator_grants::grant_active_operator_role(&pool, actor_id, "finance_admin").await;
    let state = crate::app::AppState::new(nvbes_core::config::AppConfig::default(), pool.clone());
    let app = Router::new()
        .merge(crate::revenue_center_actions::router())
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::idempotency::idempotency_guard,
        ))
        .with_state(state);
    let idempotency_key = format!("test-{}", Uuid::new_v4());
    let confirm_code =
        crate::backoffice_authorization::strong_confirmation_code("HOLD INVOICE", invoice_id);

    let first = app
        .clone()
        .oneshot(hold_request_with_key(
            workspace_id,
            invoice_id,
            actor_id,
            "finance_admin",
            &confirm_code,
            &idempotency_key,
        ))
        .await
        .expect("first request should respond");
    assert_eq!(first.status(), StatusCode::OK);

    let second = app
        .oneshot(hold_request_with_key(
            workspace_id,
            invoice_id,
            actor_id,
            "finance_admin",
            &confirm_code,
            &idempotency_key,
        ))
        .await
        .expect("second request should replay");
    assert_eq!(second.status(), StatusCode::OK);
    assert_eq!(
        second
            .headers()
            .get("idempotency-replayed")
            .and_then(|value| value.to_str().ok()),
        Some("true")
    );

    let action_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM internal_admin_revenue_actions
         WHERE tenant_id = $1 AND actor_principal_id = $2
           AND action_kind = 'hold_invoice' AND invoice_id = $3",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .bind(invoice_id)
    .fetch_one(&pool)
    .await
    .expect("action count should load");
    assert_eq!(action_count, 1);

    let audit_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM audit_events
         WHERE tenant_id = $1 AND actor_principal_id = $2
           AND action = 'revenue.invoice.held'
           AND target_type = 'billing_invoice'",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .fetch_one(&pool)
    .await
    .expect("audit count should load");
    assert_eq!(audit_count, 1);
}
